use ferrosa_wit::{ClientMode, Computer, CrimeRates, GameState, Menu, PlayerId, Plugin, Program, server};

mod accounts;
mod corporations;
mod day;
mod map;
mod mission_exe;
mod missions;
mod city;
mod state;

#[derive(Default)]
struct World {
    state: state::World,
}

impl Plugin for World {
    fn init(&mut self) {
        server::register_mode("world", &["city".to_string()], ClientMode::World, None);
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
        self.state.rate = 0;
        self.state.memos = 0;
        self.state.bills_withdrawn.clear();

        for team in state::CORPORATIONS {
            if let Some(corp) = server::get_corporation(team) {
                corp.set_funds(0);
            }

            server::set_team_door(team, true);
        }

        let c = self.state.cfg;
        server::set_crime(Some(CrimeRates {
            civ_civ: c.crimecivciv,
            civ_team: c.crimecivteam,
            team_civ: c.crimeteamciv,
            team_team: c.crimeteamteam,
            team_team_in_base: c.crimeteamteaminbase,
            kick: c.crimekick,
        }));

        self.init_mission_spots();
        server::set_game_state(GameState::Intermission);
        server::set_game_timer(0);
        self.respawn_world();
    }

    fn tick(&mut self) {
        self.tick_world();
    }

    fn program(&mut self, name: &str, computer: &Computer, _args: &str) -> Option<Box<dyn Program<Self>>> {
        match name {
            "MISSION" => Some(Box::new(mission_exe::MissionExe::new(computer))),
            _ => None,
        }
    }

    fn menus_done(&mut self) {
        for p in server::players() {
            if let Some(human) = p.human() {
                self.corp_base_menu(p.id(), human.pos());
            }

            self.corp_tab_menu(p.id());
        }

        self.copy_corp_money();
    }

    fn on_menu(&mut self, player: PlayerId, menu: Menu, button: u32) -> bool {
        if menu == Menu::EnterCity {
            return self.enter_city_menu(player, button);
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

    fn on_death(&mut self, player: PlayerId) {
        self.world_on_death(player);
    }

    fn on_leave(&mut self, player: PlayerId) {
        self.forget_applicant(player);

        for c in &mut self.state.corps {
            if c.manager == Some(player) {
                c.manager = None;
            }
        }
    }
}

ferrosa_wit::plugin!(World);
