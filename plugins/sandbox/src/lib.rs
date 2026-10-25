use ferrosa_wit::{ClientMode, Computer, GameState, PlayerId, Plugin, Program, command, server};

use phonebook::PhoneBook;

mod commands;
mod phonebook;

#[derive(Default)]
struct Sandbox;

impl Plugin for Sandbox {
    fn init(&mut self) {
        server::register_mode("sandbox", &[], ClientMode::Round, None);
        server::register_program("PHONEBOOK");
    }

    fn program(&mut self, name: &str, computer: &Computer, args: &str) -> Option<Box<dyn Program>> {
        match name {
            "PHONEBOOK" => Some(Box::new(PhoneBook::new(computer, args))),
            _ => None,
        }
    }

    fn reset(&mut self) {
        server::set_game_state(GameState::InGame);
        server::set_traffic(128, false);
    }

    fn on_join(&mut self, player: PlayerId) {
        if let Some(p) = server::get_player(player) {
            p.message("Welcome to sandbox.");
        }
    }

    fn on_chat(&mut self, player: PlayerId, message: &str) -> bool {
        let Some((name, args)) = command(message) else { return false };
        let Some(p) = server::get_player(player) else { return false };

        commands::run(&p, name, &args)
    }
}

ferrosa_wit::plugin!(Sandbox);
