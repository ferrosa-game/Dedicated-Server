use ferrosa_protocol::{Team, clientbound::game::VehicleKind};

use crate::{
    sim::{
        Sim,
        corporations::team_index,
        economy::Corporation,
        missions::{MISSION_SLOTS, Mission},
    },
    world::building::CorporationBase,
};

impl Sim {
    pub fn corporation(&self, team: Team) -> Option<Corporation> {
        team_index(team).map(|k| self.corporations[k])
    }

    pub fn set_corporation_shares(&mut self, team: Team, shares: i32) {
        if let Some(k) = team_index(team) {
            self.corporations[k].shares = shares;
        }
    }

    pub fn set_corporation_price(&mut self, team: Team, price: f32) {
        let Some(k) = team_index(team) else { return };
        self.corporations[k].price = price;

        self.updates.stock_prices = true;
    }

    pub fn corporation_funds(&self, team: Team) -> i32 {
        team_index(team).map_or(0, |k| self.corp_state[k].funds)
    }

    pub fn set_corporation_funds(&mut self, team: Team, funds: i32) {
        if let Some(k) = team_index(team) {
            self.corp_state[k].funds = funds;
        }
    }

    pub fn team_door_open(&self, team: Team) -> bool {
        team_index(team).is_some_and(|k| self.corp_state[k].door_open)
    }

    pub fn set_team_door_open(&mut self, team: Team, open: bool) {
        let Some(k) = team_index(team) else { return };

        self.set_team_door(k, open);
        self.apply_team_doors();
    }

    pub fn corporation_base(&self, team: Team) -> Option<&CorporationBase> {
        team_index(team).and_then(|k| self.world.map.level.bases.get(k))
    }

    /// A vehicle in the first free car space of the corporation's base.
    pub fn spawn_corporation_vehicle(&mut self, team: Team, kind: u32, colour: i32) -> Option<usize> {
        let kind = VehicleKind::try_from(u8::try_from(kind).ok()?).ok()?;
        self.corporation_spawn_vehicle(kind, team_index(team)?, colour)
    }

    pub fn missions(&self, team: Team) -> Option<&[Mission; MISSION_SLOTS]> {
        team_index(team).map(|k| &self.corp_state[k].missions)
    }

    /// Sets a mission slot and tells the corporation's clients.
    pub fn set_mission(&mut self, team: Team, slot: usize, mission: Mission) {
        let Some(k) = team_index(team) else { return };

        if slot >= MISSION_SLOTS {
            return;
        }

        self.corp_state[k].missions[slot] = mission;
        self.push_mission_event(k, slot);
    }
}
