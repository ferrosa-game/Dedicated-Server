use ferrosa_wit::{Base, PlayerId, Team, Vec3, server};

use crate::{World, state::CORPORATIONS};

fn in_base(base: &Base, pos: Vec3) -> bool {
    pos.x >= base.interior_min.x && pos.x < base.interior_max.x && pos.z >= base.interior_min.z && pos.z < base.interior_max.z
}

fn base_of(team: Team) -> Option<Base> {
    server::get_corporation(team).and_then(|c| c.base())
}

impl World {
    pub fn score_crime(&mut self, attacker: PlayerId, victim: PlayerId, damage: i32) {
        let (Some(a), Some(v)) = (server::get_player(attacker), server::get_player(victim)) else { return };
        let (Some(ah), Some(vh)) = (a.human(), v.human()) else { return };

        if a.bot() || v.bot() {
            return;
        }

        let victim_pos = vh.bones().first().map_or(Vec3::ZERO, |b| b.pos());
        let car = vh.last_vehicle();
        let owner = car.as_ref().and_then(|c| c.owner());
        let (a_team, v_team) = (a.team(), v.team());
        let cfg = self.state.cfg;

        let rate = if a_team == Team::Spectator {
            if owner == Some(attacker) {
                return;
            }

            if v_team == Team::Spectator { cfg.crimecivciv } else { cfg.crimecivteam }
        } else if let Some(k) = crate::state::World::team_index(a_team) {
            if base_of(CORPORATIONS[k]).is_some_and(|b| in_base(&b, victim_pos)) {
                return;
            }

            if owner.and_then(server::get_player).is_some_and(|o| o.team() == a_team) {
                return;
            }

            if v_team == Team::Spectator {
                cfg.crimeteamciv
            } else if base_of(v_team).is_some_and(|b| in_base(&b, victim_pos)) {
                cfg.crimeteamteaminbase
            } else {
                cfg.crimeteamteam
            }
        } else {
            cfg.crimecivciv
        };

        let mut k = rate as f32 / 100.0;
        let crim = v.crim_rating();

        if crim > 0 {
            let f = crim as f32 / 100.0;

            if f > 1.0 {
                k *= 0.0;
            } else {
                k *= 1.0 - f;
            }
        }

        let gain = (damage as f32 * k) as i32;
        let rating = (a.crim_rating() + gain.max(0)).min(1023);

        a.set_crim_rating(rating);

        if cfg.crimekick <= rating {
            ah.set_vitality(0);
        }
    }
}
