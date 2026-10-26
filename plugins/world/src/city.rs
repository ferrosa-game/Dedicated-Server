use ferrosa_wit::{BuildingKind, ItemKind, Mat3, ShopEntry, Vec3, server};

use crate::{World, state::CORPORATIONS};

impl World {
    pub fn map_spawn(&self) -> Vec3 {
        if server::map_name() == "test2" {
            let t = server::track_spawns().get(4).map_or(Vec3::ZERO, |s| s.0);

            return Vec3::new(t.x + 16.0, t.y + 1.0, t.z);
        }

        let Some((end, _)) = server::street_intersections(0) else { return Vec3::ZERO };
        let corners = server::intersections();
        let Some(i) = corners.get(end as usize) else { return Vec3::ZERO };
        let p = i.pos;

        Vec3::new((p.x + 96.0) + 1384.0, p.y + 1.0, p.z - 24.0)
    }

    pub fn stock_burger_shops(&mut self) {
        let stock = [ShopEntry { kind: 0, price: 20, extra: 0 }];

        for b in server::buildings().iter().filter(|b| b.kind() == BuildingKind::BurgerShop) {
            if b.shop().len() != 1 {
                b.set_shop(&stock);
            }
        }
    }

    pub fn world_crime_tick(&mut self) {
        for team in CORPORATIONS {
            if let Some(corp) = server::get_corporation(team) {
                corp.set_prints(0);
            }
        }

        self.state.bills_withdrawn.clear();

        for p in server::players() {
            p.set_items_bought(0);

            if p.vehicles_bought() > 0 {
                p.set_vehicles_bought(p.vehicles_bought() - 1);
            }

            // If they are above the buy limit, just increase the bought to make it so they cant
            if p.crim_rating() >= self.state.cfg.crimenobuy {
                p.set_items_bought(i32::MAX);
                p.set_vehicles_bought(i32::MAX);
            }
        }

        self.account_crime_tick();
    }

    pub fn expire_world_bots(&mut self) {
        let clock = server::world_time();

        for p in server::players().into_iter().filter(|p| p.bot()) {
            let (human, deadline) = (p.human(), p.bot_deadline());

            if deadline == 0 || deadline >= clock {
                if human.is_none() {
                    p.remove();
                }

                continue;
            }

            if let Some(h) = human {
                h.release_from_bot();
            }

            p.remove();
        }
    }

    pub fn place_world_memos(&mut self) {
        let s = self.map_spawn();
        let paper = String::from_utf8_lossy(&self.state.newspaper).into_owned();

        while self.state.memos < 8 {
            let pos = Vec3::new(s.x - 4.0, s.y, s.z + 1.0);
            let Some(item) = server::spawn_item_by_id(ItemKind::PaperWorld as u32, pos, Mat3::IDENTITY) else { break };

            item.write_memo(&paper);
            self.state.memos += 1;
        }
    }
}