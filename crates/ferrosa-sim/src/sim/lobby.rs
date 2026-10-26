use ferrosa_protocol::clientbound::game::{MenuType, VehicleKind};

use super::{Sim, corporations::team_index};
use crate::PlayerId;

const CLOCK_START: i32 = 2376000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SavedItem {
    pub kind: i32,
    pub a: i32,
    pub b: i32,
}

impl Sim {
    /// count_corp_players: Reset the player count on each corp struct,
    /// goverened by `rules.count_humanless_members`, works on all coprs
    pub(crate) fn count_corp_players(&mut self) {
        for c in &mut self.corp_state {
            c.player_count = 0;
        }

        let humanless = self.rules.count_humanless_members;

        for (_, p) in self.players.iter() {
            if let Some(k) = team_index(p.team)
                && (p.human.is_some() || humanless)
            {
                self.corp_state[k].player_count += 1;
            }
        }
    }

    /// check_occupied: Check if the game has players
    pub(crate) fn check_occupied(&mut self) {
        let occupied = self.players.iter().any(|(_, p)| !p.is_bot);

        if occupied && !self.occupied {
            self.reset_game();
        }

        self.occupied = occupied;
    }

    pub(crate) fn roll_weather(&mut self) {
        let angle = (((rand::random_range(0..256) as i32 + 64) as f64 * std::f64::consts::PI) * 0.25 * (1.0 / 256.0)) as f32;
        let tilt = ((rand::random_range(0..256) as f64 * std::f64::consts::PI) * (1.0 / 256.0)) as f32;

        self.world_time = CLOCK_START;
        self.world.set_sun(angle, tilt);
    }

    /// Reset players humans, items bought, and clear the world
    pub(crate) fn reset_lobby_players_and_world(&mut self) {
        for (_, player) in self.players.iter_mut() {
            player.items_bought = 0;
            player.vehicles_bought = 0;
            player.human = None;
            player.ghost_human = false;
            player.is_ready = false;
            player.menu = MenuType::Lobby;
        }

        self.clear_world();
        self.round_number += 1;
    }

    /// clear_world: Close all programs, reset bodies humans items, just clean up.
    pub(crate) fn clear_world(&mut self) {
        let gravity = self.bodies.gravity_scale;

        self.drop_all_programs();

        self.bodies = ferrosa_physics::RigidBodies::default();
        self.bodies.gravity_scale = gravity;
        self.humans = ferrosa_physics::Table::new(256);
        self.items = ferrosa_physics::Table::new(1024);
        self.vehicles = ferrosa_physics::Table::new(512);
        self.bullets.clear();
        self.npcs.clear();

        for c in &mut self.corp_state {
            c.car_space_taken.clear();
        }

        self.traffic = crate::traffic::Traffic::new(&self.world.map.streets, self.world.map.map_name == "round");
        self.events.clear();

        for clients in self.clients.values_mut() {
            clients.reset_for_round();
        }
    }



    pub(crate) fn corporation_spawn_vehicle(&mut self, kind: VehicleKind, k: usize, color: i32) -> Option<usize> {
        let spaces = self.world.map.level.bases.get(k)?.car_spaces.clone();
        let taken = &mut self.corp_state[k].car_space_taken;
        taken.resize(spaces.len(), false);

        let i = taken.iter().position(|t| !t)?;
        taken[i] = true;

        let (pos, rot) = spaces[i];
        self.spawn_vehicle(kind, color, pos, rot)
    }

    pub(crate) fn base_buy_menu(&mut self, pid: PlayerId, outside: Option<glam::Vec3>) {
        let Some(pos) = outside else { return };

        let at_base = self.rules.corp_buy_menus
            && self.players.get(pid.idx()).and_then(|p| team_index(p.team)).is_some_and(|k| self.world.map.level.bases.get(k).is_some_and(|b| b.contains(pos)));

        let Some(p) = self.players.get_mut(pid.idx()) else { return };

        if !at_base {
            p.menu = MenuType::Empty;
        } else if p.menu == MenuType::Empty {
            p.menu = MenuType::RoundCorpWeapons;
        }
    }

}
