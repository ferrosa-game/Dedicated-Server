use std::collections::HashMap;

use ferrosa_wit::{DamageCause, Human, HumanId, game_seconds, server};

struct Immortal {
    down: i32,
    blood: i32,
}

#[derive(Default)]
pub(crate) struct Immortals {
    humans: HashMap<HumanId, Immortal>,
    ticks: u32,
}

fn heal(v: &mut i32) {
    if *v <= 99 {
        *v += 1;
    }
}

impl Immortals {
    pub(crate) fn add(&mut self, human: &Human) {
        let mut health = human.health();
        health.vitality = 100;
        human.set_health(health);

        self.humans.insert(human.id(), Immortal { down: 0, blood: health.blood });
    }

    pub(crate) fn clear(&mut self) {
        self.humans.clear();
    }

    pub(crate) fn on_damage(&self, human: HumanId, cause: DamageCause, damage: i32) -> i32 {
        let Some(immortal) = self.humans.get(&human) else { return damage };

        match cause {
            DamageCause::Fall | DamageCause::Vehicle(_) => 0,
            DamageCause::Bullet(_) if immortal.down > 0 => 0,
            _ => damage,
        }
    }

    pub(crate) fn tick(&mut self) {
        self.ticks += 1;

        let every = |n: u32| self.ticks.is_multiple_of(n);
        let (health_tick, parts_tick, blood_tick) = (every(8), every(16), every(256));

        self.humans.retain(|&id, immortal| {
            let Some(human) = server::get_human(id) else { return false };
            let mut h = human.health();

            if h.vitality <= 0 {
                return false;
            }

            if immortal.down > 0 {
                immortal.down -= 1;
            }

            if h.health <= 49 {
                immortal.down = game_seconds(30);
            }

            if health_tick {
                heal(&mut h.health);
            }

            if parts_tick {
                for part in [&mut h.chest, &mut h.head, &mut h.left_arm, &mut h.right_arm, &mut h.left_leg, &mut h.right_leg] {
                    heal(part);
                }
            }

            if blood_tick {
                heal(&mut immortal.blood);
            }

            h.blood = immortal.blood;
            h.chest = h.chest.max(1);
            h.head = h.head.max(1);
            human.set_health(h);
            true
        });
    }
}
