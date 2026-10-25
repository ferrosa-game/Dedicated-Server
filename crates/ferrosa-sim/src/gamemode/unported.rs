use ferrosa_protocol::GameMode;

use super::{ModeHooks, ModeSync, rules::ModeRules};
use crate::{
    sim::Sim, time::game_seconds,
};

/// The binary's modes this server does not run (driving, racing, co-op, versus and none), with what the shared code
/// still does differently for them.
pub struct Unported(GameMode, &'static str);

pub static DRIVING: Unported = Unported(GameMode::Driving, "driving");
pub static RACING: Unported = Unported(GameMode::Racing, "racing");
pub static COOP: Unported = Unported(GameMode::CoOp, "coop");
pub static VERSUS: Unported = Unported(GameMode::Versus, "versus");
pub static NONE: Unported = Unported(GameMode::None, "none");

/// config_versus.txt (load_conf_versus).
#[derive(Clone, Copy, Debug, Default)]
pub struct VersusConfig {
    /// Seconds at the start of a versus round in which nobody can walk (sent in the initial sync).
    pub movedelay: i32,
}

impl VersusConfig {
    pub(crate) fn load(path: &std::path::Path) -> Self {
        let mut c = VersusConfig::default();

        if let Ok(text) = std::fs::read_to_string(path) {
            let num = |key: &str| text.lines().find_map(|l| l.trim().strip_prefix(key)).and_then(|v| v.trim().parse::<i32>().ok());

            c.movedelay = num("movedelay=").unwrap_or(c.movedelay);
        }

        c
    }
}

impl ModeHooks for Unported {
    fn name(&self) -> &'static str {
        self.1
    }

    fn aliases(&self) -> &'static [&'static str] {
        if self.0 == GameMode::Versus { &["v"] } else { &[] }
    }

    fn client_mode(&self) -> GameMode {
        self.0
    }

    fn map(&self) -> Option<&'static str> {
        match self.0 {
            GameMode::CoOp => Some("round"),
            GameMode::None => Some("test2"),
            _ => None,
        }
    }

    fn rules(&self, sim: &Sim) -> ModeRules {
        let base = ModeRules::default();

        match self.0 {
            GameMode::Racing => ModeRules { reset_traffic: None, intermission_lobby: true, ..base },
            GameMode::CoOp => ModeRules { coop_traffic: true, intermission_lobby: true, ..base },
            GameMode::Versus => ModeRules {
                team_damage: 50,
                corp_buy_menus: true,
                intermission_lobby: true,
                move_delay: Some(game_seconds(sim.modes.versus.movedelay)),
                ..base
            },
            _ => base,
        }
    }

    fn joining_money(&self, _sim: &Sim, money: i32) -> i32 {
        if self.0 == GameMode::Versus { 250 } else { money }
    }

    fn sync(&self, sim: &Sim) -> ModeSync {
        let movedelay = (self.0 == GameMode::Versus).then_some(sim.modes.versus.movedelay as u8);
        ModeSync { weekly_enabled: false, weekday: sim.world.weekday as u8, versus_movedelay: movedelay }
    }
}
