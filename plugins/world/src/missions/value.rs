//! What a mission is worth to a corporation: its reward, raised by its manager's rating and how many members it has.

use super::DAILY_MISSIONS;
use crate::World;

/// Each day step a corporation's open missions grow by its manager's bonus plus 10% a member, at most 100%.
const MEMBER_BONUS: i32 = 10;

const MAX_MEMBERS: i32 = 10;

impl World {
    /// The percentage a corporation's manager adds to its mission values: their corporate rating past 1000, a point
    /// per 100, up to 1000.
    pub(super) fn manager_bonus(&self, t: usize) -> i32 {
        self.state.corps[t].manager.map_or(0, |p| ((self.state.corp_rating(p) - 1000) / 100).clamp(0, 1000))
    }

    /// set_team_manager in world mode: the day's untouched missions worth something to the corporation are worth
    /// their reward plus the new manager's bonus.
    pub fn scale_missions_for_manager(&mut self, t: usize, rating: i32) {
        let bonus = ((rating - 1000) / 100).clamp(0, 1000);

        for m in self.state.missions.missions.iter_mut().take(DAILY_MISSIONS) {
            let v = &mut m.teams[t].value;

            if m.state == 0 && *v > 0 {
                *v = bonus.wrapping_mul(m.reward) / 100 + m.reward;
            }
        }
    }

    /// increment_manager_rating: each of the day's untouched missions worth something to the corporation grows by
    /// the manager's bonus and 10% a member (100% from ten members).
    pub fn increment_manager_rating(&mut self, t: usize) {
        let bonus = self.manager_bonus(t);
        let members = self.corp_player_count(t);
        let rate = bonus + if members < MAX_MEMBERS { members * MEMBER_BONUS } else { MAX_MEMBERS * MEMBER_BONUS };

        for m in self.state.missions.missions.iter_mut().take(DAILY_MISSIONS) {
            let v = &mut m.teams[t].value;

            if m.state == 0 && *v > 0 {
                *v = m.reward.wrapping_mul(rate) / 100 + m.reward;
            }
        }
    }
}
