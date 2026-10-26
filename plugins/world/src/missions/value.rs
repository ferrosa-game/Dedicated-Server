use ferrosa_wit::server;

use crate::World;

impl World {
    pub(super) fn manager_bonus(&self, t: usize) -> i32 {
        self.state.corps[t].manager.and_then(server::get_player).map_or(0, |p| ((p.corp_rating() - 1000) / 100).clamp(0, 1000))
    }

    pub fn scale_missions_for_manager(&mut self, t: usize, rating: i32) {
        let bonus = ((rating - 1000) / 100).clamp(0, 1000);

        for m in self.state.missions.missions.iter_mut().take(7) {
            let v = &mut m.teams[t].value;

            if m.state == 0 && *v > 0 {
                *v = bonus.wrapping_mul(m.reward) / 100 + m.reward;
            }
        }
    }

    pub fn increment_manager_rating(&mut self, t: usize) {
        let bonus = self.manager_bonus(t);
        let members = self.corp_player_count(t);
        let rate = bonus + if members < 10 { members * 10 } else { 100 };

        for m in self.state.missions.missions.iter_mut().take(7) {
            let v = &mut m.teams[t].value;

            if m.state == 0 && *v > 0 {
                *v = m.reward.wrapping_mul(rate) / 100 + m.reward;
            }
        }
    }
}
