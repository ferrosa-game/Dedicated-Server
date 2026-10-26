use ferrosa_wit::server;

use super::names::CORPORATION_NAMES;
use crate::{World, state::CORPORATIONS};

impl World {
    pub fn file_trade(&mut self, m: usize, team: usize) {
        self.state.missions.missions[m].teams[team].uploaded = true;

        if self.state.missions.missions[m].state > 1 {
            return;
        }

        let value = self.state.missions.missions[m].teams[team].value;

        if value <= 0 {
            {
                let t = &mut self.state.missions.missions[m].teams[team];
                t.lines = vec!["TRADE".into()];
                t.word = "Trade".into();
            }

            for i in 0..6 {
                let mission = &mut self.state.missions.missions[m];

                if i == team || mission.teams[i].value <= 0 {
                    continue;
                }

                let state = mission.state;
                let v = mission.teams[i].value;
                let ti = &mut mission.teams[i];

                if state == 0 {
                    ti.lines.clear();
                }

                ti.lines.push(format!("{} HAS THE FILE  VALUE: ${}", CORPORATION_NAMES[team], v));
                ti.word = "Trade".into();
                let memo = format!("{} has obtained the file for the\n{} project", CORPORATION_NAMES[team], mission.names[3]);
                mission.teams[team].lines.push(format!("{} WANTS THE FILE", CORPORATION_NAMES[i]));
                server::place_team_memo(CORPORATIONS[i], &memo);
            }

            self.state.missions.missions[m].state = 1;

            return;
        }

        self.state.corps[team].money += value;
        let mission = &mut self.state.missions.missions[m];

        for (i, t) in mission.teams.iter_mut().enumerate() {
            if i == team {
                t.lines = vec!["MISSION SUCCESS".into(), format!("VALUE: ${value}")];
                t.word = "Success".into();
            } else {
                t.lines = vec!["MISSION CLOSED".into()];
                t.word = "Closed".into();
            }
        }

        mission.state = 2;

        if let Some(corp) = server::get_corporation(CORPORATIONS[team])
            && corp.intel() > 0
        {
            corp.set_intel(1);
        }
    }
}
