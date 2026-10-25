use ferrosa_protocol::clientbound::game::events::{Event, ServerEvent, explosion::EventExplosion};
use glam::Vec3;

use crate::{PlayerId, input::ItemInput, items::state::ItemState, plugin::DamageCause, sim::Sim};

const GRENADE_PUSH_RANGE: f32 = 8.0;
const PUSH_FALLOFF: f32 = 3.0 / 32.0;
const MIN_PUSH_WEIGHT: f32 = 0.125;

impl Sim {
    pub(crate) fn grenade_logic(&mut self, id: usize) {
        let item = self.items.get_mut(id).unwrap();
        let (rising, holder) = (item.pressed(ItemInput::Secondary), item.parent_human);

        let holder_player = (holder != -1).then(|| self.humans.get(holder as usize)).flatten().and_then(|h| h.player);

        let ItemState::Grenade { pin, fuse, primer } = &mut item.state else { return };

        if *pin > 0 {
            if rising {
                *fuse = 240;
                *pin = 0;

                if holder != -1 {
                    *primer = holder_player;
                }
            }

            return;
        }

        let c = *fuse;

        if c <= 0 {
            return;
        }

        if c == 240 {
            if rising {
                *pin = 1;
                *primer = None;
            } else if holder == -1 {
                *fuse = 240 - 1;
            }

            return;
        }

        *fuse = c - 1;

        if c - 1 != 1 {
            return;
        }

        item.despawn_time = 0;
        let pos = self.bodies.get(item.body).map_or(item.pos, |b| b.pos);
        self.events.push(Event { tick_created: self.tick, kind: ServerEvent::Explosion(EventExplosion { size: 0, pos }) });
        let primer = *primer;
        self.grenade_explosion(pos, primer);
    }

    /// grenade_explosion: kills every human whose pelvis is within 4 of it and throws the bones of everyone within 8
    /// outwards. The primer is scored for each player killed other than themselves.
    fn grenade_explosion(&mut self, at: Vec3, primer: Option<PlayerId>) {
        let mut killed = Vec::new();
        let in_range: Vec<usize> = self.humans.iter().filter(|(_, h)| h.bones[0].pos.distance(at) < 4.0).map(|(id, _)| id).collect();
        let plugins = self.plugins;

        for id in in_range {
            let Some(vitality) = self.humans.get(id).map(|h| h.health.vitality) else { continue };
            let damage = plugins.on_damage(self, id, DamageCause::Explosion(primer), None, vitality.max(0));
            let Some(human) = self.humans.get_mut(id) else { continue };

            human.health.vitality -= damage;

            if damage > 0 && human.health.vitality <= 0 {
                killed.extend(human.player);
            }
        }

        for (_, human) in self.humans.iter_mut() {
            let dist = human.bones[0].pos.distance(at);

            if dist < 8.0 {
                for bone in &human.bones {
                    if let Some(body) = self.bodies.get_mut(bone.body) {
                        body.vel += blast_push(body.pos - at, bone.mass);
                    }
                }
            }
        }

        let Some(primer) = primer else { return };

        for victim in killed {
            self.score_hurt(Some(primer), Some(victim), 100);
        }
    }
}

fn blast_push(offset: Vec3, mass: f32) -> Vec3 {
    let dist = offset.length();
    let weight = (8.0 / (mass + 8.0)).max(MIN_PUSH_WEIGHT);
    let falloff = (GRENADE_PUSH_RANGE - dist) * PUSH_FALLOFF;
    offset.normalize_or_zero() * (falloff * falloff * 1.5 * weight)
}
