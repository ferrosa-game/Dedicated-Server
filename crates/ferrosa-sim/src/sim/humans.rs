use ferrosa_physics::{
    RotMatrix,
    rotation::rot_matrix_to_quaternion,
};
use ferrosa_protocol::clientbound::game::{
    HumanAppearance, ItemKind, OwnHumanData, ServerHumanObject,
    events::{
        Event, ServerEvent,
        bullet_hit::EventBulletHit,
        bullet_hole::EventBulletHole,
        sound::{EventSound, Sound},
    },
};
use glam::Vec3;

use super::{Sim, items::touchables};
use crate::{
    PlayerId,
    human::{
        Human,
        create::create_human,
        inventory::detach_item,
        physics::{HumanOutput, sync_bones},
        simulation::{HumanTick, simulate_human},
    },
    plugin::DamageCause,
    world::skeleton::BoneId,
};

impl Sim {
    pub(crate) fn spawn_human(&mut self, pos: Vec3, rot: &RotMatrix, player: Option<PlayerId>) -> Option<usize> {
        let player = player.and_then(|p| self.players.get(p.idx()));

        create_human(&mut self.humans, &mut self.bodies, pos, rot, player)
    }

    /// player_simulation: hands each player's latest controls to their human.
    pub(crate) fn player_simulation(&mut self) {
        let held = self.rules.move_delay.is_some_and(|ticks| self.round_elapsed < ticks);

        for (_, player) in self.players.iter_mut() {
            if player.spawn_timer > 0 {
                player.spawn_timer -= 1;
            }

            let Some(h) = player.human.and_then(|id| self.humans.get_mut(id)) else { continue };

            h.stocks = player.stocks;
            player.vitality = h.health.vitality.max(0);

            if h.vehicle.is_some() {
                if matches!(player.control_mode, 2 | 3) {
                    let c = &player.controls;
                    h.gear_x_input = c.gear_x;
                    h.strafe_input = c.strafe;
                    h.gear_y_input = c.gear_y;
                    h.walk_input = c.walk;
                    h.look_yaw = c.look_yaw;
                    h.look_pitch = c.look_pitch;
                    h.free_look_yaw = c.free_look_yaw;
                    h.free_look_pitch = c.free_look_pitch;
                }
            } else if player.control_mode == 1 {
                let c = &player.controls;
                h.gear_x_input = c.gear_x;
                h.gear_y_input = c.gear_y;
                h.strafe_input = c.strafe;
                h.walk_input = c.walk;
                h.look_yaw = c.look_yaw;
                h.look_pitch = c.look_pitch;
                h.free_look_yaw = c.free_look_yaw;
                h.free_look_pitch = c.free_look_pitch;
                h.client_body_yaw = c.body_yaw;
                h.jump_spin_yaw = c.spin_yaw;
                h.jump_spin_pitch = c.spin_pitch;

                if held {
                    h.strafe_input = 0.0;
                    h.walk_input = 0.0;
                }
            } else {
                h.look_yaw = h.view_yaw;
                h.look_pitch = h.view_pitch;
            }

            h.input_flags = player.input_bits;
            h.movement_mode = player.zoom_level as i32;
        }
    }

    pub(crate) fn sync_humans(&mut self) {
        for id in self.humans.ids() {
            let Some(h) = self.humans.get_mut(id) else { continue };

            sync_bones(h, &mut self.bodies, &self.world.map);

            let (lo, hi) = (glam::IVec3::from_array(h.grid_min), glam::IVec3::from_array(h.grid_max));

            self.spawn_set_items_in(lo, hi);
        }
    }

    pub(crate) fn simulate_humans(&mut self) {
        for id in self.humans.ids() {
            let car = self.humans.get(id).and_then(|h| h.vehicle).and_then(|v| self.vehicles.get(v)).and_then(|v| usize::try_from(v.traffic_car).ok());
            let driver = car.and_then(|c| self.traffic.cars.get(c)).is_some_and(|c| c.human == id as i32);
            self.humans.get_mut(id).unwrap().traffic_driver = driver;
        }

        let map = &mut self.world.map;

        let mut out = Vec::new();
        let mut touch = touchables!(self, occupied_seats(&self.humans));
        let mut deleted = Vec::new();

        let keep_bodies = self.rules.keep_bodies;

        for id in self.humans.ids() {
            let others = later_humans(&self.humans, id);
            let h = self.humans.get_mut(id).unwrap();

            if let HumanTick::Delete = simulate_human(id, h, &mut self.bodies, map, &mut touch, &others, &mut out, self.tick, &mut self.noise_seed, keep_bodies)
            {
                deleted.push(id);
            }
        }

        crate::human::grab::grab_hands(&mut self.humans, &mut self.bodies, self.tick);

        self.apply_human_outputs(out);
        self.apply_team_doors();

        for id in deleted {
            self.delete_human(id);
        }
    }

    fn deal_crush(&mut self, human: usize, cause: DamageCause, bone: BoneId, damage: i32) {
        let plugins = self.plugins;
        let damage = plugins.on_damage(self, human, cause, Some(bone), damage);
        let Some(h) = self.humans.get_mut(human) else { return };
        let landed = h.pos;

        match (cause, bone) {
            (DamageCause::Fall, BoneId::ThighLeft) => h.health.left_leg -= damage,
            (DamageCause::Fall, BoneId::ThighRight) => h.health.right_leg -= damage,
            _ => {
                let before = h.health.vitality;
                h.health.vitality -= damage;

                if let DamageCause::Vehicle(driver) = cause
                    && before > 0
                    && h.health.vitality <= 0
                {
                    let (pos, victim) = (h.pos, h.player);
                    let e = EventSound { sound_type: Sound::BodyHit as u8, pos, volume: 1.0, pitch: 0.5 };
                    self.events.push(Event { tick_created: self.tick, kind: ServerEvent::Sound(e) });

                    if let Some(driver) = driver {
                        self.score_run_over(driver, victim);
                    }
                }
            }
        }

        if cause == DamageCause::Fall && damage > 0 {
            let e = EventSound { sound_type: Sound::BodyHit as u8, pos: landed, volume: 1.0, pitch: 0.5 };

            self.events.push(Event { tick_created: self.tick, kind: ServerEvent::Sound(e) });
        }
    }

    pub(crate) fn apply_human_outputs(&mut self, out: Vec<HumanOutput>) {
        for o in out {
            let kind = match o {
                HumanOutput::Glass(b) => ServerEvent::BulletHole(EventBulletHole {
                    area: b.area,
                    block_x: b.block.x,
                    block_y: b.block.y,
                    block_z: b.block.z,
                    cell: b.cell,
                    face: b.face,
                    pos: b.pos,
                    vel: b.vel,
                }),
                HumanOutput::Blood(p) => ServerEvent::BulletHit(EventBulletHit { unk: 0, hit_type: 3, pos: p, normal: p }),
                HumanOutput::ReleasePlayer(pid) => {
                    self.settle_death(pid);
                    continue;
                }
                HumanOutput::Sound { sound, pos, volume, pitch } => ServerEvent::Sound(EventSound { sound_type: sound as u8, pos, volume, pitch }),
                HumanOutput::Damage { human, cause, bone, damage } => {
                    self.deal_crush(human, cause, bone, damage);
                    continue;
                }
                HumanOutput::DoorProbe { player, start, end, pos } => {
                    self.team_door_probe(player, start, end, pos);
                    continue;
                }
            };

            self.events.push(Event { tick_created: self.tick, kind });
        }
    }

    pub(crate) fn delete_human(&mut self, id: usize) {
        let Some(mut human) = self.humans.remove(id) else { return };
        let mut touch = touchables!(self, occupied_seats(&self.humans));

        for slot in 0..human.inventory.len() {
            while human.inventory[slot].count > 0 {
                let item = human.inventory[slot].items[human.inventory[slot].count as usize - 1] as usize;
                let before = human.inventory[slot].count;
                detach_item(&mut human, &mut self.bodies, &mut touch, item, slot);

                if human.inventory[slot].count == before {
                    break;
                }
            }
        }

        if let Some(player) = human.player.and_then(|p| self.players.get_mut(p.idx()))
            && player.human == Some(id)
        {
            player.human = None;

            let id = player.player_id;
            self.update_player_event(id);
        }

        for joint in &human.joints {
            self.bodies.remove_bond(joint.bond);
        }

        for bone in &human.bones {
            self.bodies.remove(bone.body);
        }
    }

    pub(crate) fn human_objects(&self) -> Vec<ServerHumanObject> {
        self.humans
            .iter()
            .map(|(id, h)| {
                let root = &h.bones[0];
                ServerHumanObject {
                    slot: id as u16,
                    human_id: id as u16,
                    appearance: Some(HumanAppearance {
                        player_id: h.player.map_or(-1, |p| p.0 as i32),
                        customization: h.customization,
                        wanted: h.player.and_then(|p| self.players.get(p.idx())).map_or(0, |p| wanted_group(p.crim_rating)),
                    }),
                    vehicle: h.vehicle.map_or(-1, |v| v as i32),
                    vehicle_seat: h.seat as i32,
                    alive: h.health.vitality > 0,
                    bleeding: h.bleeding,
                    pos: root.pos,
                    rot: rot_matrix_to_quaternion(&root.rot),
                    bones: std::array::from_fn(|i| rot_matrix_to_quaternion(&h.bones[i + 1].networked_rot)),
                }
            })
            .collect()
    }

    pub(crate) fn own_human_data(&self) -> std::collections::HashMap<usize, (OwnHumanData, [Vec<i32>; 7])> {
        self.players
            .iter()
            .filter_map(|(idx, p)| {
                let id = p.human?;
                let h = self.humans.get(id)?;
                Some((
                    idx,
                    (
                        OwnHumanData {
                            human_id: id as i32,
                            view_yaw: h.view_yaw,
                            view_pitch: h.view_pitch,
                            yaw_offset: h.yaw_offset,
                            pitch_offset: h.pitch_offset,
                            body_yaw: h.body_yaw,
                            is_standing: h.is_standing,
                            pain: h.pain,
                            action_type: h.action_type,
                            action_duration: h.action_duration,
                            action_hand: h.action_hand,
                            action_slot: h.action_slot,
                            progress_bar: h.progress_bar,
                            health: [
                                h.health.health,
                                h.health.chest,
                                h.health.head,
                                h.health.left_arm,
                                h.health.right_arm,
                                h.health.left_leg,
                                h.health.right_leg,
                            ],
                            stamina: h.stamina,
                            max_stamina: h.max_stamina,
                            head_vel: h.bones[3].vel,
                        },
                        std::array::from_fn(|k| {
                            let slot = &h.inventory[k];
                            slot.items[..slot.count as usize]
                                .iter()
                                .filter_map(|&i| self.items.get(i as usize))
                                .map(|item| {
                                    let kind = item.item_type as i32;

                                    if k > 1 {
                                        return kind;
                                    }

                                    let ty = &self.item_types[kind as usize];
                                    let mut shown = kind;

                                    if let Some(&mag) = item.children.first() {
                                        if ty.is_gun {
                                            let empty = self.items.get(mag).is_some_and(|m| m.state.left() == 0);
                                            shown |= if empty { 0x300 } else { 0x100 };
                                        } else if item.item_type == ItemKind::BriefcaseOpen {
                                            shown |= 0x100;
                                        }
                                    }

                                    if ty.magazine_ammo > 0 { shown | (item.state.left() << 8) } else { shown }
                                })
                                .collect()
                        }),
                    ),
                ))
            })
            .collect()
    }
}

fn later_humans(humans: &ferrosa_physics::Table<Human>, id: usize) -> Vec<crate::human::physics::OtherHuman> {
    humans.iter().filter(|&(k, h)| k > id && h.vehicle.is_none()).map(|(_, h)| crate::human::physics::OtherHuman::of(h)).collect()
}

pub(crate) fn occupied_seats(humans: &ferrosa_physics::Table<Human>) -> Vec<(usize, usize)> {
    humans.iter().filter_map(|(_, h)| h.vehicle.map(|v| (v, h.seat))).collect()
}

fn wanted_group(crim_rating: i32) -> i32 {
    match crim_rating {
        1..=99 => 1,
        r if r >= 100 => 2,
        _ => 0,
    }
}
