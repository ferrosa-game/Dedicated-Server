use ferrosa_protocol::Team;
use wasmtime::component::Resource;

use super::VehicleRef;
use crate::{
    bindings::ferrosa::plugin::types::{self, Base, CarSpace, Mission},
    host::HostState,
};

pub struct CorporationRef(pub(crate) Team);

impl HostState {
    pub(crate) fn corporation_handle(&mut self, team: Team) -> wasmtime::Result<Resource<CorporationRef>> {
        Ok(self.table.push(CorporationRef(team))?)
    }
}

impl types::HostCorporation for HostState {
    fn team(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<types::Team> {
        Ok(self.table.get(&corporation)?.0.into())
    }

    fn shares(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<i32> {
        let team = self.table.get(&corporation)?.0;
        Ok(self.sim()?.corporation(team).map_or(0, |c| c.shares))
    }

    fn set_shares(&mut self, corporation: Resource<CorporationRef>, shares: i32) -> wasmtime::Result<()> {
        let team = self.table.get(&corporation)?.0;
        self.sim_mut()?.set_corporation_shares(team, shares);
        Ok(())
    }

    fn price(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<f32> {
        let team = self.table.get(&corporation)?.0;
        Ok(self.sim()?.corporation(team).map_or(0.0, |c| c.price))
    }

    fn set_price(&mut self, corporation: Resource<CorporationRef>, price: f32) -> wasmtime::Result<()> {
        let team = self.table.get(&corporation)?.0;
        self.sim_mut()?.set_corporation_price(team, price);
        Ok(())
    }

    fn base(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<Option<Base>> {
        let team = self.table.get(&corporation)?.0;

        Ok(self.sim()?.corporation_base(team).map(|b| Base {
            interior_min: b.interior_min.into(),
            interior_max: b.interior_max.into(),
            vault_min: b.vault_min.into(),
            vault_max: b.vault_max.into(),
            table_orientation: b.table_orientation,
            table: b.table.into(),
            spawn: b.spawn.into(),
            car_spaces: b.car_spaces.iter().map(|&(pos, rot)| CarSpace { pos: pos.into(), rot: rot.into() }).collect(),
        }))
    }

    fn spawn_vehicle(&mut self, corporation: Resource<CorporationRef>, kind: u32, colour: i32) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let team = self.table.get(&corporation)?.0;
        let Some(v) = self.sim_mut()?.spawn_corporation_vehicle(team, kind, colour) else { return Ok(None) };
        self.vehicle_handle(v).map(Some)
    }

    fn funds(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<i32> {
        let team = self.table.get(&corporation)?.0;
        Ok(self.sim()?.corporation_funds(team))
    }

    fn set_funds(&mut self, corporation: Resource<CorporationRef>, funds: i32) -> wasmtime::Result<()> {
        let team = self.table.get(&corporation)?.0;
        self.sim_mut()?.set_corporation_funds(team, funds);
        Ok(())
    }

    fn missions(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<Vec<Mission>> {
        let team = self.table.get(&corporation)?.0;
        Ok(self.sim()?.missions(team).map_or_else(Vec::new, |m| m.iter().map(|&m| m.into()).collect()))
    }

    fn set_mission(&mut self, corporation: Resource<CorporationRef>, slot: u32, mission: Mission) -> wasmtime::Result<()> {
        let team = self.table.get(&corporation)?.0;
        self.sim_mut()?.set_mission(team, slot as usize, mission.into());
        Ok(())
    }

    fn drop(&mut self, corporation: Resource<CorporationRef>) -> wasmtime::Result<()> {
        self.table.delete(corporation)?;
        Ok(())
    }
}
