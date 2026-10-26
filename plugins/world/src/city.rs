use ferrosa_wit::{BuildingKind, ItemKind, Mat3, ShopEntry, Vec3, server};

use crate::{World, state::CORPORATIONS};

/// The city itself: where players enter it, and the shops and ratings that tick over through the day.
///
/// On test2 reset_game puts the spawn by the fifth train spawn, 16 east and 1 up.
const TEST2_SPAWN_TRAIN: usize = 4;
const TEST2_SPAWN_EAST: f32 = 16.0;
const TEST2_SPAWN_UP: f32 = 1.0;

/// load_map's spawn: one end of the first street, moved 1480 east, 1 up and 24 north.
const SPAWN_STREET: i32 = 0;
const SPAWN_EAST_A: f32 = 96.0;
const SPAWN_EAST_B: f32 = 1384.0;
const SPAWN_UP: f32 = 1.0;
const SPAWN_NORTH: f32 = 24.0;

const BURGER_PRICE: i32 = 20;

impl World {
    /// Where civilians enter the city: on test2 beside the fifth train spawn, otherwise off the end of the first
    /// street.
    pub fn map_spawn(&self) -> Vec3 {
        if server::map_name() == "test2" {
            let t = server::track_spawns().get(TEST2_SPAWN_TRAIN).map_or(Vec3::ZERO, |s| s.0);

            return Vec3::new(TEST2_SPAWN_EAST + t.x, TEST2_SPAWN_UP + t.y, t.z);
        }

        let Some((end, _)) = server::street_intersections(SPAWN_STREET) else { return Vec3::ZERO };
        let corners = server::intersections();
        let Some(i) = corners.get(end as usize) else { return Vec3::ZERO };
        let p = i.pos;

        Vec3::new((SPAWN_EAST_A + p.x) + SPAWN_EAST_B, SPAWN_UP + p.y, p.z - SPAWN_NORTH)
    }

    /// The burger shops' one line, topped back up every tick until they close for the day.
    pub fn stock_burger_shops(&mut self) {
        let stock = [ShopEntry { kind: 0, price: BURGER_PRICE, extra: 0 }];

        for b in server::buildings().iter().filter(|b| b.kind() == BuildingKind::BurgerShop) {
            if b.shop().len() != 1 {
                b.set_shop(&stock);
            }
        }
    }

    /// Every quarter hour: each corporation's prints and every player's purchases are forgotten, and criminal
    /// ratings decay.
    pub fn world_crime_tick(&mut self) {
        for team in CORPORATIONS {
            if let Some(corp) = server::get_corporation(team) {
                corp.set_prints(0);
            }
        }

        self.state.bills_withdrawn.clear();

        for p in server::players() {
            p.set_items_bought(0);
        }

        self.account_crime_tick();
    }
}
/// The world keeps 8 memos at the spawn, 4 to the west and 1 to the south of it.
const MEMOS: i32 = 8;

const MEMO_WEST: f32 = 4.0;

const MEMO_SOUTH: f32 = 1.0;

impl World {
    /// Bots go when they lose their human, and when their deadline passes they leave with what they hold.
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

    /// The memos at the spawn, topped back up to 8.
    pub fn place_world_memos(&mut self) {
        let s = self.map_spawn();
        let paper = String::from_utf8_lossy(&self.state.newspaper).into_owned();

        while self.state.memos < MEMOS {
            let pos = Vec3::new(s.x - MEMO_WEST, s.y, MEMO_SOUTH + s.z);
            let Some(item) = server::spawn_item_by_id(ItemKind::PaperWorld as u32, pos, Mat3::IDENTITY) else { break };

            item.write_memo(&paper);
            self.state.memos += 1;
        }
    }
}
