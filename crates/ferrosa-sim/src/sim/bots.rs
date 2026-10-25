use std::f64;

use ferrosa_protocol::{
    clientbound::game::GameState,
    serverbound::game::actions::{GameAction, InventoryAction},
};
use glam::{Vec2, Vec3, Vec3Swizzles};

use super::Sim;
use crate::{input::Input, time::game_seconds};
use crate::{PlayerId, player::BotTarget, world::trace::line_intersect_level};

/// How far the view may turn from the body, either way.
const MAX_LOOK: f64 = 7.0 * f64::consts::PI / 8.0;

/// The difference from `a` to `b` taken the short way, the way player_ai wraps it: `b` is moved up a turn when `a`
/// is half a turn past it, then the difference has `a` moved up a turn when still half a turn out.
fn wrap(a: f32, b: f32) -> f32 {
    let t = if ((a - b) as f64) >= 180.0_f64.to_radians() { ((b as f64) + 360.0_f64.to_radians()) as f32 } else { b };
    let d = t - a;
    if (d as f64) >= 180.0_f64.to_radians() { t - ((a as f64 + 360.0_f64.to_radians()) as f32) } else { d }
}

impl Sim {
    /// The bots' part of server_main: player_ai for every bot while the round is on.
    pub(crate) fn run_bots(&mut self) {
        let bots: Vec<PlayerId> = self.players.iter().filter(|(_, p)| p.is_bot).map(|(_, p)| p.player_id).collect();

        for pid in bots {
            self.player_ai(pid);
        }
    }

    fn push_bot_action(&mut self, pid: PlayerId, a: u16) {
        if let Some(p) = self.players.get_mut(pid.idx()) {
            p.actions.push(GameAction::Inventory(InventoryAction { a, b: 1, c: 0 }));
        }
    }

    /// player_ai: a bot's controls for the tick. It reloads, then on foot walks its waypoints and turns towards and
    /// shoots at what it has seen; seated, it drives its traffic car's route or watches from its window and shoots.
    pub(crate) fn player_ai(&mut self, pid: PlayerId) {
        let Some(p) = self.players.get_mut(pid.idx()) else { return };

        p.controls.gear_x = 0.0;
        p.controls.strafe = 0.0;
        p.controls.gear_y = 0.0;
        p.controls.walk = 0.0;
        p.input_bits = 0;

        if p.bot.delay > 0 {
            p.bot.delay -= 1;
            return;
        }

        let Some(hid) = p.human else { return };
        self.bot_reload(pid, hid);

        let Some(h) = self.humans.get(hid) else { return };

        match h.vehicle {
            None => self.bot_on_foot(pid, hid),
            Some(veh) => {
                let seat = h.seat;
                self.players.get_mut(pid.idx()).unwrap().control_mode = 2;

                if seat != 0 {
                    self.bot_seated(pid, hid, veh, seat);
                } else if self.gamestate != GameState::Intermission {
                    self.player_ai_control_vehicle(pid, veh);
                } else {
                    self.players.get_mut(pid.idx()).unwrap().controls.walk = -1.0;
                }

                self.bot_check_wreck(pid, veh);
            }
        }
    }

    /// A wrecked vehicle is left after 180 ticks.
    fn bot_check_wreck(&mut self, pid: PlayerId, veh: usize) {
        let wrecked = self.vehicles.get(veh).is_some_and(|v| v.health <= 0);
        let p = self.players.get_mut(pid.idx()).unwrap();

        if wrecked {
            p.bot.wrecked_ticks += 1;

            if p.bot.wrecked_ticks > game_seconds(3) {
                p.input_bits |= Input::Grab.mask();
            }
        } else {
            p.bot.wrecked_ticks = 0;
        }
    }

    /// The gun in the right hand: an empty chamber is filled from the magazine, an empty magazine swapped for one in the
    /// left hand or a pocket, now and then.
    fn bot_reload(&mut self, pid: PlayerId, hid: usize) {
        let Some(human) = self.humans.get(hid) else { return };

        if human.action_type != -1 || human.inventory[0].count <= 0 {
            return;
        }

        let Some(gun) = self.items.get(human.inventory[0].items[0] as usize) else { return };

        if gun.state.left() != 0 {
            return;
        }

        let gun_type = gun.item_type as usize;

        if let Some(&mag) = gun.children.first() {
            let rounds = self.items.get(mag).map_or(0, |m| m.state.left());

            if rounds <= 0 {
                if rand::random_bool(1.0 / 32.0) {
                    self.push_bot_action(pid, 1);
                }
            } else if rand::random_bool(1.0 / 32.0) {
                self.players.get_mut(pid.idx()).unwrap().input_bits |= Input::Reload.mask();
            }

            return;
        }

        if human.inventory[1].count <= 0 {
            let pockets: Vec<(usize, Option<usize>)> = (3..7).map(|s| (s, (human.inventory[s].count > 0).then(|| human.inventory[s].items[0] as usize))).collect();

            for (s, first) in pockets {
                let Some(kind) = first.and_then(|i| self.items.get(i)).map(|i| i.item_type as usize) else { continue };

                if self.item_types[kind].can_mount_to[gun_type] != 0 && !rand::random::<bool>() {
                    self.push_bot_action(pid, s as u16 + 1);
                }
            }

            return;
        }

        let rounds = self.items.get(human.inventory[1].items[0] as usize).map_or(0, |m| m.state.left());

        if rand::random_bool(1.0 / 32.0) {
            self.push_bot_action(pid, if rounds <= 0 { 2 } else { 1 });
        }
    }

    /// The bot's human (`hid`) looks around every other tick: each living enemy within 128 in front of its chest and in
    /// plain sight is remembered or seen again, then its memories fade and it picks the nearest it is sure of.
    fn bot_find_targets(&mut self, pid: PlayerId, hid: usize) {
        if (self.tick as i32 ^ pid.i32()) & 1 != 0 {
            return;
        }

        let Some(me) = self.humans.get(hid) else { return };
        let (chest, front) = (me.bones[3].pos, me.bones[3].rot[2]);
        let Some(team) = self.players.get(pid.idx()).map(|p| p.team) else { return };
        let zombie = self.players.get(pid.idx()).is_some_and(|p| p.bot.is_zombie);
        let map = &self.world.map;

        for j in self.humans.ids() {
            let h = self.humans.get(j).unwrap();

            if h.health.vitality <= 0 {
                continue;
            }

            let Some(their) = h.player.and_then(|p| self.players.get(p.idx())) else { continue };

            if their.team == team {
                continue;
            }

            if chest.distance(h.bones[3].pos) >= 128.0 {
                continue;
            }

            let bot = &self.players.get(pid.idx()).unwrap().bot;
            let found = bot.targets.iter().rposition(|t| t.human == j as i32);
            let bone = if h.vehicle.is_some() { 3 } else { (self.tick & 15) as usize };
            let seen = h.bones[bone].pos;
            let dir = seen - chest;
            let range = dir.length();
            let facing = dir.dot(front);

            if facing >= 0.0 {
                continue;
            }

            if !zombie && line_intersect_level(&map.ground, &map.level.area, &map.level.meshes, chest, seen).is_some() {
                continue;
            }

            let p = self.players.get_mut(pid.idx()).unwrap();
            let k = match found {
                Some(k) => k,
                None => {
                    if p.bot.targets.len() >= 16 {
                        continue;
                    }

                    p.bot.targets.push(BotTarget { human: j as i32, pos: seen, awareness: 0.0, timer: game_seconds(10) });
                    p.bot.targets.len() - 1
                }
            };
            let t = &mut p.bot.targets[k];
            let mut aw = 1.0 + t.awareness;

            if aw > 16.0 {
                aw = 16.0;
            }

            t.awareness = aw;
            t.pos *= 0.875;
            let unsure = (16.0 - aw) * 0.125;
            let spread = unsure * 0.25 + (0.25 * range) * 0.125;
            let out = crate::human::arms::calculate_spread_vector(&mut self.noise_seed, spread, 0.0);
            let ahead = out + seen + h.bones[0].vel * 4.0;
            let t = &mut self.players.get_mut(pid.idx()).unwrap().bot.targets[k];
            t.pos = ahead * 0.125 + t.pos;
        }

        let my_pos = self.humans.get(hid).unwrap().pos;
        let humans = &self.humans;
        let p = self.players.get_mut(pid.idx()).unwrap();
        let targets = &mut p.bot.targets;
        let mut i = 0;

        while i < targets.len() {
            targets[i].awareness *= 31.0 / 32.0;

            if 0.875 > targets[i].awareness {
                let last = targets.len() - 1;
                targets[i] = targets[last];
                targets.truncate(last);
            } else {
                i += 1;
            }
        }

        p.bot.target = 0;
        let pos_of = |t: &BotTarget| humans.get(t.human as usize).map_or(Vec3::ZERO, |h| h.pos);
        let first = targets.first().map_or(Vec3::ZERO, pos_of);
        let mut best = my_pos.distance(first);

        for (k, t) in targets.iter().enumerate().skip(1) {
            if t.awareness <= 8.0 {
                continue;
            }

            let r = my_pos.distance(pos_of(t));

            if best > r {
                p.bot.target = k as i32;
                best = r;
            }
        }
    }

    /// A bot on foot: the view follows the body's turns, a fallen bot only crouches, and otherwise it walks to its
    /// waypoints, goes to look at an enemy it barely saw, and turns to aim at and shoot one it is sure of.
    fn bot_on_foot(&mut self, pid: PlayerId, hid: usize) {
        self.bot_find_targets(pid, hid);
        let h = self.humans.get(hid).unwrap();
        let (body_yaw, pelvis_up, view_yaw, yaw_offset, pos) = (h.body_yaw, h.bones[0].rot[1].y, h.view_yaw, h.yaw_offset, h.pos);
        let tick = self.tick;
        let p = self.players.get_mut(pid.idx()).unwrap();
        p.control_mode = 1;
        let turn = wrap(p.controls.body_yaw, body_yaw);
        let mut look = p.controls.look_yaw - turn;
        p.controls.body_yaw = body_yaw;

        if -MAX_LOOK > look as f64 {
            look = -157.5_f32.to_radians();
        } else if look as f64 > MAX_LOOK {
            look = 157.5_f32.to_radians();
        }

        p.controls.look_yaw = look;

        if 0.9 > pelvis_up {
            if (tick & 0x7f) > 64 {
                p.input_bits |= Input::Crouch.mask();
            }

            return;
        }

        let mut yaw_to = p.bot.idle_yaw;

        if p.bot.waypoint < p.bot.waypoint_count {
            p.controls.walk = 1.0;
            let wp = p.bot.waypoint_pos(p.bot.waypoint);
            let flat = Vec2::new(wp.x - pos.x, wp.z - pos.z);
            let d = flat.length();
            let to = flat.normalize_or_zero();
            yaw_to = (to.x as f64).atan2((-to.y) as f64) as f32;

            if 2.0 > d {
                p.bot.waypoint += 1;

                if !(p.bot.waypoint < p.bot.waypoint_count) {
                    p.bot.waypoint = 0;
                }
            }
        }

        let mut attack = false;

        if !p.bot.targets.is_empty() {
            let t = p.bot.targets[p.bot.target as usize];
            let mut aw = t.awareness;

            if 2.0 > aw && 2.0 > (t.pos.y - pos.y).abs() {
                if rand::random_ratio(1, 32) {
                    let p = self.players.get_mut(pid.idx()).unwrap();
                    p.bot.waypoint = 0;
                    p.bot.set_waypoint(0, t.pos);
                    p.bot.waypoint_count = 1;
                }

                let p = self.players.get(pid.idx()).unwrap();
                aw = p.bot.targets.get(p.bot.target as usize).map_or(0.0, |t| t.awareness);
                attack = !p.bot.targets.is_empty() && aw > 12.0;
            } else {
                attack = aw > 12.0;
            }
        }

        let (yaw_to, pitch_to) = if attack { self.bot_attack_on_foot(pid, hid) } else { (yaw_to, 0.0) };
        let p = self.players.get_mut(pid.idx()).unwrap();
        let mut facing = (view_yaw + yaw_offset) + p.controls.look_yaw;

        if -180.0_f64.to_radians() > facing as f64 {
            facing = (facing as f64 + 360.0_f64.to_radians()) as f32;
        }

        if facing as f64 > 180.0_f64.to_radians() {
            facing = (facing as f64 - 360.0_f64.to_radians()) as f32;
        }

        let d = wrap(facing, yaw_to);
        p.controls.look_yaw += d * 0.5;
        p.controls.look_pitch = (0.125 * (pitch_to - p.controls.look_pitch)) * 0.5 + p.controls.look_pitch;
        p.zoom_level = 1;

        if p.bot.targets.first().is_some_and(|t| t.awareness > 8.0) {
            p.zoom_level = 0;
        }
    }

    /// An on-foot bot sure of its target: it walks at it (a zombie keeps walking), stops within 32 and, aimed within 64
    /// with no teammate in the way, fires now and then. The heading and pitch to turn to.
    fn bot_attack_on_foot(&mut self, pid: PlayerId, hid: usize) -> (f32, f32) {
        let p = self.players.get(pid.idx()).unwrap();
        let t = p.bot.targets[p.bot.target as usize];
        let (zombie, pitch_now, team) = (p.bot.is_zombie, p.controls.look_pitch, p.team);
        let h = self.humans.get(hid).unwrap();
        let eye = h.bones[7].pos;
        let aim = t.pos - eye - Vec3::Y * 0.25;
        let d = aim.length();
        let yaw = (aim.x as f64).atan2((-aim.z) as f64) as f32;
        let horiz = (aim.xz().length_squared() as f64).sqrt();
        let pitch = ((-aim.y) as f64).atan2(horiz) as f32;
        let p = self.players.get_mut(pid.idx()).unwrap();

        if zombie {
            p.controls.walk = 1.0;
        } else if 32.0 > d {
            p.controls.walk = 0.0;
        }

        if d >= 64.0 {
            return (yaw, pitch);
        }

        let yaw_diff = wrap(h.view_yaw, yaw).abs();
        let pitch_diff = wrap(pitch_now, pitch).abs();

        if yaw_diff >= 0.125 || pitch_diff >= 0.125 {
            return (yaw, pitch);
        }

        let head = &h.bones[9];
        let (start, up) = (head.pos, head.rot[1]);
        let end = up * -64.0 + start;

        if self.teammate_in_line(hid, team, h.bones[3].pos, start, end, 128.0) {
            return (yaw, pitch);
        }

        if self.bot_gun_loaded(hid) && rand::random_ratio(1, 4) {
            self.players.get_mut(pid.idx()).unwrap().input_bits |= Input::LeftMouse.mask();
        }

        (yaw, pitch)
    }

    /// Whether a teammate's human within `range` of `chest` is in the line of fire `start..end`.
    fn teammate_in_line(&self, hid: usize, team: ferrosa_protocol::Team, chest: Vec3, start: Vec3, end: Vec3, range: f32) -> bool {
        self.humans.ids().into_iter().filter(|&j| j != hid).any(|j| {
            let h = self.humans.get(j).unwrap();
            let Some(p) = h.player.and_then(|p| self.players.get(p.idx())) else { return false };

            if p.team != team {
                return false;
            }

            range > chest.distance(h.bones[3].pos) && crate::human::damage::trace_ray_human(h, start, end, 0.25).is_some()
        })
    }

    /// bot_gun_loaded, its the gun being held loaded
    fn bot_gun_loaded(&self, hid: usize) -> bool {
        let h = self.humans.get(hid).unwrap();
        h.inventory[0].count > 0 && self.items.get(h.inventory[0].items[0] as usize).is_some_and(|g| g.state.left() > 0)
    }

    /// A passenger bot looks out of its window, or turns towards an enemy it is sure of in its vehicle's frame and,
    /// aimed within 32 with neither a teammate nor its own vehicle's body in the way, fires.
    fn bot_seated(&mut self, pid: PlayerId, hid: usize, veh: usize, seat: usize) {
        self.bot_find_targets(pid, hid);

        let mut yaw = match seat {
            1 => 45.0_f32.to_radians(),
            2 | 4 => -135.0_f32.to_radians(),
            3 | 5 => 135.0_f32.to_radians(),
            _ => 0.0,
        };

        let mut pitch = 0.0;
        let player = self.players.get(pid.idx()).unwrap();
        let team = player.team;
        let aimed_at = player.bot.targets.get(player.bot.target as usize).filter(|t| !player.bot.targets.is_empty() && t.awareness > 8.0).copied();

        if let Some(t) = aimed_at {
            let human = self.humans.get(hid).unwrap();
            let eye = human.bones[9].pos;

            let aim = t.pos - eye - Vec3::Y * 0.25;
            let d = aim.length();
            let n = aim.normalize_or_zero();

            let [r0, r1, r2] = self.vehicles.get(veh).map_or(ferrosa_physics::rotation::IDENTITY, |v| v.rot);

            let lx = r0.dot(n);
            let ly = r1.dot(n);
            let lz = n.dot(r2);

            yaw = (lx as f64).atan2((-lz) as f64) as f32;

            let horiz = ((lx * lx + lz * lz) as f64).sqrt();
            pitch = ((-ly) as f64).atan2(horiz) as f32;

            let p = self.players.get(pid.idx()).unwrap();

            if 32.0 > d && 0.125 > wrap(p.controls.look_yaw, yaw).abs() && 0.125 > wrap(p.controls.look_pitch, pitch).abs() {
                self.bot_seated_fire(pid, hid, veh, team, n);
            }
        }

        let p = self.players.get_mut(pid.idx()).unwrap();
        p.controls.look_yaw = ((yaw - p.controls.look_yaw) * 0.125) * 0.5 + p.controls.look_yaw;
        p.controls.look_pitch = ((pitch - p.controls.look_pitch) * 0.125) * 0.5 + p.controls.look_pitch;
    }

    /// The seated bot's shot along its gun's magazine, when the gun points at the enemy (`n`, a unit vector).
    fn bot_seated_fire(&mut self, pid: PlayerId, hid: usize, veh: usize, team: ferrosa_protocol::Team, n: Vec3) {
        let h = self.humans.get(hid).unwrap();
        let Some(mag) = self.items.get(h.inventory[0].items[0] as usize).and_then(|g| self.items.get(g.first_child)) else { return };
        let Some(body) = self.bodies.get(mag.body) else { return };
        let r2 = body.rot[2];
        let along = n.dot(r2);

        if along >= -399.0 / 400.0 {
            return;
        }

        let start = mag.pos;
        let end = r2 * -64.0 + start;

        if self.teammate_in_line(hid, team, h.bones[3].pos, start, end, 64.0) {
            return;
        }

        let own_body =
            self.vehicles.get(veh).and_then(|v| crate::vehicle::physics::trace_vehicle_parts(v, &self.vehicle_types[v.kind as usize], start, end, false));

        if own_body.is_some_and(|hit| matches!(hit.part, crate::vehicle::physics::VehiclePart::Body(_))) {
            return;
        }

        if self.bot_gun_loaded(hid) {
            self.players.get_mut(pid.idx()).unwrap().input_bits |= Input::LeftMouse.mask();
        }
    }

    /// player_ai_control_vehicle: the driver lets its traffic car's AI (made aggressive) steer and sets the gas from how
    /// far the car is below the speed the AI wants, slowing for sharp turns
    fn player_ai_control_vehicle(&mut self, pid: PlayerId, veh: usize) {
        let Some(car) = self.vehicles.get(veh).and_then(|v| usize::try_from(v.traffic_car).ok()) else { return };
        self.traffic.cars[car].is_aggressive = 1;

        crate::traffic::ai::ai_traffic_car(&mut self.traffic, &self.world.map.streets, car);

        let vehicle = self.vehicles.get(veh).unwrap();
        let Some(car) = usize::try_from(vehicle.traffic_car).ok() else { return };

        let traffic_car = &self.traffic.cars[car];
        let (steer, target, stuck) = (traffic_car.steer, traffic_car.target_speed, traffic_car.stuck);
        let (vel, r2) = (vehicle.vel, vehicle.rot[2]);

        let player = self.players.get_mut(pid.idx()).unwrap();
        player.controls.gear_x = 2.0;
        player.controls.gear_y = -1.0;
        player.controls.strafe = steer;

        let gas = (vel.dot(r2) + target) * 8.0;
        let gas = if gas > 1.0 { 1.0 } else { gas };
        player.controls.walk = if 0.05 > target { -1.0 } else { (gas as f64 / ((steer.abs() as f64) + 1.0)) as f32 };

        player.input_bits = 0;

        if stuck > game_seconds(3) {
            player.controls.walk = 1.0;
            player.controls.strafe = 0.0;
            player.controls.gear_x = 2.0;
            player.controls.gear_y = 1.0;

            if stuck > game_seconds(5) {
                player.controls.walk = -1.0;
            }
        }
    }
}
