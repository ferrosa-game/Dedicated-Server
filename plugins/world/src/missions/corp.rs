use ferrosa_wit::server;

use crate::{World, state::CORPORATIONS};


impl World {
    pub fn expire_corp_missions(&mut self, clock: i32) {
        for team in CORPORATIONS {
            let Some(corp) = server::get_corporation(team) else { continue };

            for (slot, m) in corp.missions().into_iter().enumerate() {
                if !m.active || m.done || m.deadline != clock {
                    continue;
                }

                server::despawn_missions(m.id);

                if !corp.missions().get(slot).is_some_and(|m| m.done) {
                    corp.settle_mission(slot as u32, false, 0);
                }
            }
        }
    }

    pub fn keep_mission_items(&mut self) {
        for team in CORPORATIONS {
            let Some(corp) = server::get_corporation(team) else { continue };

            for m in corp.missions() {
                if !m.active {
                    continue;
                }

                if let Some(item) = m.item.and_then(server::get_item)
                    && item.despawn_time() > 0
                {
                    item.set_despawn_time(65535);
                }
            }
        }
    }
}
