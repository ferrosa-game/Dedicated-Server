use std::f32::consts::FRAC_PI_2;

use ferrosa_wit::{GameState, Mat3, Menu, PlayerId, Team, Vec3, clock_time, game_minutes, server};

use crate::{World, state::CORPORATIONS};

const CLOCK_START: i32 = clock_time(8, 0);
const MISSIONS_AT: i32 = CLOCK_START + 60;
const DAY_STEP: i32 = clock_time(0, 15);
const LAST_DAY_STEP: i32 = clock_time(17, 0) - 1;
const DAY_END: i32 = clock_time(19, 0) - 1;

impl World {
    pub fn start_world(&mut self) {
        server::set_game_state(GameState::InGame);
        server::set_game_timer(self.state.max_time);
        server::set_world_time(CLOCK_START);
        self.state.clock_rate = 10;

        crate::map::create_trains();
        server::set_traffic(self.state.cfg.traffic.max(0) as u32, false);
        crate::map::guard_the_labs();

        self.state.memo_timers = [0; CORPORATIONS.len()];
    }

    pub fn tick_world(&mut self) {
        self.expire_world_bots();

        if server::get_game_state() == GameState::Intermission {
            self.start_world();
        }

        if server::get_game_state() != GameState::InGame {
            return;
        }

        let clock = server::world_time() + self.state.clock_rate;
        server::set_world_time(clock);

        if clock == MISSIONS_AT {
            self.start_world_missions();
        }

        if clock % DAY_STEP == 0 && clock <= LAST_DAY_STEP {
            for k in 0..CORPORATIONS.len() {
                self.increment_manager_rating(k);
            }

            for k in 0..CORPORATIONS.len() {
                if self.corp_player_count(k) > 0 {
                    if self.state.memo_timers[k] <= 0 {
                        self.generate_team_mission_memo(k);
                    } else {
                        self.state.memo_timers[k] -= 1;
                    }
                }
            }
        }

        if clock > DAY_END {
            return self.end_world_day();
        }

        self.stock_burger_shops();
        self.expire_corp_missions(clock);

        if clock % game_minutes(5) == 0 {
            self.world_save_tick();
        }

        if clock % DAY_STEP == 0 {
            self.world_crime_tick();
        }

        self.place_world_memos();

        for p in server::players() {
            if !p.bot() && p.human().is_none() {
                p.set_menu(Menu::EnterCity);
            }
        }

        self.keep_mission_items();
        self.world_account_sync();

        for p in server::players() {
            let timer = p.spawn_timer();

            if timer > 0 {
                p.set_spawn_timer(timer - 1);
            }
        }
    }

    pub fn corp_player_count(&self, k: usize) -> i32 {
        server::players().iter().filter(|p| p.team() == CORPORATIONS[k]).count() as i32
    }

    pub fn enter_city_menu(&mut self, pid: PlayerId, button: u32) -> bool {
        let Some(p) = server::get_player(pid) else { return false };

        if p.menu() != Menu::EnterCity || p.human().is_some() {
            return false;
        }

        if p.spawn_timer() != 0 {
            return true;
        }

        match button {
            1 => self.set_world_team(pid, Team::Spectator),
            2 => {}
            _ => return true,
        }

        if p.crim_rating() < self.state.cfg.crimenospawn {
            self.enter_city(pid);
        }

        true
    }

    pub fn enter_city(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };
        let team = crate::state::World::team_index(p.team());

        let (pos, yaw) = match team {
            None => {
                let s = self.map_spawn();
                let spread = |r: i32| ((r as f32 * (14.0)) * 0.5) * (1.0 / 256.0);
                let x = spread(rand::random_range(0..256)) + s.x;
                let z = spread(rand::random_range(0..256)) + s.z;

                (Vec3::new(x, s.y, z), FRAC_PI_2)
            }
            Some(k) => {
                let Some(base) = server::get_corporation(CORPORATIONS[k]).and_then(|c| c.base()) else { return };
                let table = Vec3::new(base.table.x, base.table.y - 0.5, base.table.z);

                let yaw = base.table_orientation + FRAC_PI_2;
                let rot = Mat3::from_yaw(yaw);

                let (r0, r2) = (rot.x, rot.z);

                let x = ((-4.0 * r2.x) + table.x) + (r0.x * 18.0);
                let y = (r0.y * 18.0) + ((r2.y * -4.0) + table.y);
                let z = (r0.z * 18.0) + (table.z + (r2.z * -4.0));

                (Vec3::new(x, y, z), yaw)
            }
        };

        let Some(human) = server::spawn_human(pos, Mat3::from_yaw(yaw), Some(pid)) else { return };

        p.set_human(&human);
        human.set_view_yaw(yaw);
        self.restore_inventory(pid);
    }
}
