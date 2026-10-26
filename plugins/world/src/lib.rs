use ferrosa_wit::{ClientMode, Computer, GameState, Menu, PlayerId, Plugin, Program, Vec3, server};

mod accounts;
mod bank;
mod corporations;
mod crime;
mod day;
mod map;
mod mission_exe;
mod missions;
mod city;
mod state;
mod store;

#[derive(Default)]
struct World {
    state: state::World,
}

impl Plugin for World {
    fn init(&mut self) {
        server::register_mode("world", &["city".to_string()], ClientMode::World, Some("test2"));
        server::register_program("MISSION");
    }

    fn reset(&mut self) {
        if self.state.round_number == 0 {
            self.state.cfg = state::Config::load();

            for c in &mut self.state.corps {
                c.manager = None;
            }
        }

        self.state.round_number += 1;
        self.state.clock_rate = 0;
        self.state.memos = 0;
        self.state.bills_withdrawn.clear();

        server::reset_lobby();
        server::set_shares_follow_human(true);

        for team in state::CORPORATIONS {
            if let Some(corp) = server::get_corporation(team) {
                corp.set_funds(0);
            }

            server::set_team_door(team, true);
        }


        self.init_mission_spots();
        server::set_game_state(GameState::Intermission);
        server::set_game_timer(0);
        self.respawn_world();
    }

    fn tick_after_traffic(&mut self) {
        self.tick_world();
    }

    fn program(&mut self, name: &str, computer: &Computer, _args: &str) -> Option<Box<dyn Program<Self>>> {
        match name {
            "MISSION" => Some(Box::new(mission_exe::MissionExe::new(computer))),
            _ => None,
        }
    }

    fn base_menus(&mut self, player: PlayerId, outside: Option<Vec3>) {
        if let Some(pos) = outside {
            self.corp_base_menu(player, pos);
        }

        self.corp_tab_menu(player);
    }

    fn menus_done(&mut self) {
        self.copy_corp_money();
    }

    fn joining_money(&mut self, money: i32) -> i32 {
        money.max(self.state.cfg.mincash)
    }

    fn on_menu(&mut self, player: PlayerId, menu: Menu, button: u32) -> bool {
        if menu == Menu::EnterCity {
            return self.enter_city_menu(player, button);
        }

        let open = server::get_player(player).is_some_and(|p| p.menu() == menu && p.human().is_some());

        if !open {
            return false;
        }

        if menu == Menu::WorldBank || menu == Menu::WorldBank2 {
            self.bank_menu_action(player, button as i32);

            return true;
        }

        let action = match menu {
            Menu::WorldCorpApplication => 20,
            Menu::WorldCorpTabs => 21,
            Menu::WorldCorpHiring => 22,
            Menu::WorldCorpFiring => 23,
            Menu::WorldCorpTeam => 24,
            Menu::WorldCorpRequisition => 25,
            _ => return false,
        };

        self.corp_menu_action(player, action, button as i32)
    }

    fn on_join(&mut self, player: PlayerId) {
        self.restore_account(player);
    }

    fn on_hurt(&mut self, attacker: PlayerId, victim: PlayerId, damage: i32) {
        self.score_crime(attacker, victim, damage);
    }

    fn on_death(&mut self, player: PlayerId) {
        self.world_on_death(player);
    }

    fn on_leave(&mut self, player: PlayerId) {
        self.save_player_account(player);
        self.state.accounts.save();
        self.state.bills_withdrawn.remove(&player);
        self.forget_applicant(player);

        for c in &mut self.state.corps {
            if c.manager == Some(player) {
                c.manager = None;
            }
        }
    }
}

ferrosa_wit::plugin!(World);
