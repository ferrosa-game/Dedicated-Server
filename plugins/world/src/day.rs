use ferrosa_wit::{GameState, Mat3, Menu, PlayerId, Team, Vec3, server};

use crate::{World, state::CORPORATIONS};

/// The world clock starts each day at 8 AM and moves 10 a tick; the day's missions are handed out a world second
/// later, the corporations' memos and manager pay every quarter hour until 5 PM, and the day ends past 7 PM.
const WORLD_HOUR: i32 = 60 * 60;
const CLOCK_START: i32 = 8 * WORLD_HOUR;
const CLOCK_RATE: i32 = 10;
const MISSIONS_AT: i32 = CLOCK_START + 60;
const DAY_STEP: i32 = WORLD_HOUR / 4;
const LAST_DAY_STEP: i32 = 17 * WORLD_HOUR - 1;
const DAY_END: i32 = 19 * WORLD_HOUR - 1;
const SAVE_PERIOD: i32 = 18000;

/// enter_city: a civilian lands somewhere in the spawn area facing west; a corporation member 18 along their table
/// and 4 back from it, facing along it.
const SPAWN_AREA: f32 = 16.0;
const SPAWN_MARGIN: f32 = 2.0;
const CIVILIAN_YAW: f32 = -std::f32::consts::FRAC_PI_2;
const TABLE_ALONG: f32 = 18.0;
const TABLE_BACK: f32 = -4.0;
const TABLE_DROP: f32 = 0.5;
const QUARTER_TURN: f32 = std::f32::consts::FRAC_PI_2;
const ENTER_AS_CIVILIAN: u32 = 1;
const ENTER_AS_TEAM: u32 = 2;

impl World {
    /// start_world: the day begins in game at 8 AM with the clock running, the trains and lab guards placed and the
    /// traffic asked for.
    pub fn start_world(&mut self) {
        server::set_game_state(GameState::InGame);
        server::set_game_timer(self.state.max_time);
        server::set_world_time(CLOCK_START);
        self.state.rate = CLOCK_RATE;

        crate::map::create_trains();
        server::set_traffic(self.state.cfg.traffic.max(0) as u32, false);
        crate::map::guard_the_labs();

        self.state.memo_timers = [0; CORPORATIONS.len()];
    }

    /// logic_world: the clock, then whatever the time has come round to.
    pub fn tick_world(&mut self) {
        self.expire_world_bots();

        if server::get_game_state() == GameState::Intermission {
            self.start_world();
        }

        if server::get_game_state() != GameState::InGame {
            return;
        }

        let clock = server::world_time() + self.state.rate;
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

        if clock % SAVE_PERIOD == 0 {
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

        for timer in self.state.spawn_timers.values_mut() {
            if *timer > 0 {
                *timer -= 1;
            }
        }
    }

    /// How many players a corporation has, which the server counts for the lobby.
    pub fn corp_player_count(&self, k: usize) -> i32 {
        server::players().iter().filter(|p| p.team() == CORPORATIONS[k]).count() as i32
    }

    /// The enter-city menu (1) of a player without a human: button 1 enters as a civilian, button 2 as they are, once
    /// their spawn wait is over and while their criminal rating is under crimenospawn.
    pub fn enter_city_menu(&mut self, pid: PlayerId, button: u32) -> bool {
        let Some(p) = server::get_player(pid) else { return false };

        if p.menu() != Menu::EnterCity || p.human().is_some() {
            return false;
        }

        if self.state.spawn_timer(pid) != 0 {
            return true;
        }

        match button {
            ENTER_AS_CIVILIAN => self.set_world_team(pid, Team::Spectator),
            ENTER_AS_TEAM => {}
            _ => return true,
        }

        if p.crim_rating() < self.state.cfg.crimenospawn {
            self.enter_city(pid);
        }

        true
    }

    /// A human for the player: a civilian somewhere in the spawn area facing west, a corporation member beside their
    /// table facing along it.
    pub fn enter_city(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };
        let team = crate::state::World::team_index(p.team());

        let (pos, yaw) = match team {
            None => {
                let s = self.map_spawn();
                let spread = |r: i32| ((r as f32 * (SPAWN_AREA - SPAWN_MARGIN)) * 0.5) * (1.0 / 256.0);
                let x = spread(rand::random_range(0..256)) + s.x;
                let z = spread(rand::random_range(0..256)) + s.z;

                (Vec3::new(x, s.y, z), CIVILIAN_YAW)
            }
            Some(k) => {
                let Some(base) = server::get_corporation(CORPORATIONS[k]).and_then(|c| c.base()) else { return };
                let table = Vec3::new(base.table.x, base.table.y - TABLE_DROP, base.table.z);
                let yaw = base.table_orientation + QUARTER_TURN;
                let rot = Mat3::from_yaw(yaw);
                let (r0, r2) = (rot.x, rot.z);
                let x = ((TABLE_BACK * r2.x) + table.x) + (r0.x * TABLE_ALONG);
                let y = (r0.y * TABLE_ALONG) + ((r2.y * TABLE_BACK) + table.y);
                let z = (TABLE_ALONG * r0.z) + (table.z + (r2.z * TABLE_BACK));

                (Vec3::new(x, y, z), yaw)
            }
        };

        let Some(human) = server::spawn_human(pos, Mat3::from_yaw(yaw), Some(pid)) else { return };

        p.set_human(&human);
        human.set_view_yaw(yaw);
        self.restore_inventory(pid);
    }
}
