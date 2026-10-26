use ferrosa_world::skeleton::BoneId;

use super::Sim;
use crate::{PlayerId, human::damage::damage_human};

impl Sim {
    /// handle_team_kill: the team damage is logged and dealt to the player's head, and the player's details resent.
    pub fn handle_team_kill(&mut self, player: PlayerId, damage: i32) {
        let Some(player) = self.players.get(player.idx()) else { return };

        if player.god_mode || damage <= 0 || !self.rules.punish_team_kills {
            return;
        }

        println!("[Sim] {}({}) team damage: {damage}", player.username, player.phone_number);

        let share = self.rules.team_damage;

        if let Some(h) = player.human.and_then(|h| self.humans.get_mut(h)) {
            damage_human(h, BoneId::Head, damage * share / 100);
        }

        let (update, round) = (player.make_update_player_event(self.tick), player.make_update_financials_event(self.tick));
        self.events.push(update);
        self.events.push(round);
    }

    /// The scoring of one player hurting another: a teammate is punished (not in eliminator), and the mode scores
    /// the hit, as bullet_simulation does it.
    pub(crate) fn score_hurt(&mut self, attacker: Option<PlayerId>, victim: Option<PlayerId>, damage: i32) {
        let (Some(attacker), Some(victim)) = (attacker, victim) else { return };

        if attacker == victim {
            return;
        }

        self.plugins.on_hurt(self, attacker, victim, damage);

        if self.same_team(attacker, victim) {
            self.handle_team_kill(attacker, damage);
        }

    }

    pub(crate) fn same_team(&self, a: PlayerId, b: PlayerId) -> bool {
        matches!((self.players.get(a.idx()), self.players.get(b.idx())), (Some(p), Some(q)) if p.team == q.team)
    }

    /// human_collide_vehicle's scoring of a run-over: the driver's criminal rating goes up for another player's
    /// human, and a driver who ran over a teammate (or themselves) is punished.
    pub(crate) fn score_run_over(&mut self, driver: PlayerId, victim: Option<PlayerId>) {
        let Some(victim) = victim else { return };

        if driver != victim {
            self.plugins.on_hurt(self, driver, victim, 100);
        }

        if self.same_team(driver, victim) {
            self.handle_team_kill(driver, 100);
        }
    }
}
