use ferrosa_protocol::Team;
use ferrosa_world::building::{self, ShopEntry};
use wasmtime::component::Resource;

use crate::{
    bindings::ferrosa::plugin::types::{self, BuildingKind, Mat3, Vec3},
    host::HostState,
    types::VolumeRef,
};

pub struct BuildingRef(pub(crate) usize);

impl HostState {
    pub(crate) fn building_handle(&mut self, id: usize) -> wasmtime::Result<Resource<BuildingRef>> {
        Ok(self.table.push(BuildingRef(id))?)
    }
}

fn kind(record: building::BuildingKind) -> BuildingKind {
    match record {
        building::BuildingKind::Other => BuildingKind::Other,
        building::BuildingKind::CorporationBase => BuildingKind::CorporationBase,
        building::BuildingKind::CarDealer => BuildingKind::CarDealer,
        building::BuildingKind::Lab => BuildingKind::Lab,
        building::BuildingKind::ClothingStore => BuildingKind::ClothingStore,
        building::BuildingKind::Bank => BuildingKind::Bank,
        building::BuildingKind::GunStore => BuildingKind::GunStore,
        building::BuildingKind::BurgerShop => BuildingKind::BurgerShop,
    }
}

impl types::HostBuilding for HostState {
    fn id(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<u32> {
        Ok(self.table.get(&building)?.0 as u32)
    }

    fn kind(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<BuildingKind> {
        let id = self.table.get(&building)?.0;
        Ok(kind(self.sim()?.building(id).map_or(building::BuildingKind::Other, |b| b.kind)))
    }

    fn interior_min(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or(glam::Vec3::ZERO, |b| b.interior_min).into())
    }

    fn interior_max(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or(glam::Vec3::ZERO, |b| b.interior_max).into())
    }

    fn pos(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or(glam::Vec3::ZERO, |b| b.pos).into())
    }

    fn rot(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Mat3> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or_else(Default::default, |b| b.rot).into())
    }

    fn team(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Option<types::Team>> {
        let id = self.table.get(&building)?.0;
        let team = self.sim()?.building(id).map(|b| b.team).filter(|&t| t >= 0);

        Ok(team.and_then(|t| Team::try_from(t).ok()).map(Into::into))
    }

    fn lobby(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or(glam::Vec3::ZERO, |b| b.lobby).into())
    }

    fn spot(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Option<u32>> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).and_then(|b| u32::try_from(b.mission_spot).ok()))
    }

    fn streets(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<(i32, i32)> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or((-1, -1), |b| b.streets))
    }

    fn contains(&mut self, building: Resource<BuildingRef>, pos: Vec3, margin: f32) -> wasmtime::Result<bool> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building_contains(id, pos.into(), margin))
    }

    fn shop(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec<types::ShopEntry>> {
        let id = self.table.get(&building)?.0;
        let entries = self.sim()?.building_shop(id).iter().map(|e| types::ShopEntry { kind: e.kind, price: e.price, extra: e.extra });

        Ok(entries.collect())
    }

    fn set_shop(&mut self, building: Resource<BuildingRef>, entries: Vec<types::ShopEntry>) -> wasmtime::Result<()> {
        let id = self.table.get(&building)?.0;
        let entries = entries.into_iter().map(|e| ShopEntry { kind: e.kind, price: e.price, extra: e.extra }).collect();
        self.sim_mut()?.set_building_shop(id, entries);

        Ok(())
    }

    fn car_spawn(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building(id).map_or(glam::Vec3::ZERO, |b| b.car_spawn).into())
    }

    fn car_counter(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&building)?.0;
        Ok(self.sim()?.building_car_counter(id))
    }

    fn set_car_counter(&mut self, building: Resource<BuildingRef>, next: i32) -> wasmtime::Result<()> {
        let id = self.table.get(&building)?.0;
        self.sim_mut()?.set_building_car_counter(id, next);

        Ok(())
    }

    fn patrol(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec<types::PatrolSpot>> {
        let id = self.table.get(&building)?.0;
        let spots = self.sim()?.building_patrol(id);
        let spots: Vec<types::PatrolSpot> = spots
            .iter()
            .map(|s| types::PatrolSpot { pos: s.pos.into(), facing: s.facing as f32 * std::f32::consts::FRAC_PI_2, waypoints: s.waypoints.iter().map(|&w| w.into()).collect() })
            .collect();

        Ok(spots)
    }

    fn computers(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<Vec<types::BuildingComputer>> {
        let id = self.table.get(&building)?.0;
        let computers = self.sim()?.building_computers(id);
        let computers: Vec<types::BuildingComputer> =
            computers.iter().map(|&(cell, volume)| types::BuildingComputer { cell: glam::Vec3::new(cell.x as f32, cell.y as f32, cell.z as f32).into(), volume }).collect();

        Ok(computers)
    }

    fn set_computer_volume(&mut self, building: Resource<BuildingRef>, index: u32, volume: Resource<VolumeRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&building)?.0;
        let volume = self.table.get(&volume)?.0;
        self.sim_mut()?.set_building_computer_volume(id, index as usize, volume);

        Ok(())
    }

    fn drop(&mut self, building: Resource<BuildingRef>) -> wasmtime::Result<()> {
        self.table.delete(building)?;
        Ok(())
    }
}
