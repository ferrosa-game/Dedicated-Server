use ferrosa_wit::{ClientMode, GameState, Plugin, Rules, server};

/// Our own mode: no game mode logic, always in game, with the test commands open to everyone. Clients are told it is
/// round mode.
#[derive(Default)]
struct Sandbox;

impl Plugin for Sandbox {
    fn init(&mut self) {
        server::register_mode("sandbox", &[], ClientMode::Round, None);
    }

    fn rules(&mut self) -> Rules {
        Rules { test_commands: true, ..server::default_rules() }
    }

    fn reset(&mut self) {
        server::set_game_state(GameState::InGame);
    }
}

ferrosa_wit::plugin!(Sandbox);
