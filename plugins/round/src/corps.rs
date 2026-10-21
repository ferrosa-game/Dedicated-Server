use std::ops::{Index, IndexMut};

use ferrosa_wit::Team;

use crate::TEAMS;

#[derive(Clone, Copy, Default)]
pub(crate) struct Corps<T>(pub(crate) [T; 3]);

impl<T> Corps<T> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Team, &T)> {
        TEAMS.into_iter().zip(&self.0)
    }

    pub(crate) fn map<U>(self, f: impl FnMut(T) -> U) -> Corps<U> {
        Corps(self.0.map(f))
    }
}

impl<T> Index<Team> for Corps<T> {
    type Output = T;

    fn index(&self, team: Team) -> &T {
        &self.0[team as usize]
    }
}

impl<T> IndexMut<Team> for Corps<T> {
    fn index_mut(&mut self, team: Team) -> &mut T {
        &mut self.0[team as usize]
    }
}
