use wasmtime::component::Resource;

use super::BodyRef;
use crate::{
    bindings::ferrosa::plugin::types::{self, Mat3, Vec3},
    host::HostState,
};

pub struct VehicleRef(pub(crate) usize);

impl HostState {
    pub(crate) fn vehicle_handle(&mut self, id: usize) -> wasmtime::Result<Resource<VehicleRef>> {
        Ok(self.table.push(VehicleRef(id))?)
    }
}

impl types::HostVehicle for HostState {
    fn id(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<u32> {
        Ok(self.table.get(&vehicle)?.0 as u32)
    }

    fn exists(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<bool> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle(id).is_some())
    }

    fn kind(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<u32> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle(id).map_or(0, |v| v.kind as u32))
    }

    fn colour(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle(id).map_or(0, |v| v.color))
    }

    fn health(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle(id).map_or(0, |v| v.health))
    }

    fn body(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Option<Resource<BodyRef>>> {
        let id = self.table.get(&vehicle)?.0;
        let Some(body) = self.sim()?.vehicle(id).map(|v| v.body) else { return Ok(None) };
        self.body_handle(body).map(Some)
    }

    fn wheels(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Vec<Resource<BodyRef>>> {
        let id = self.table.get(&vehicle)?.0;
        let bodies: Vec<usize> = self.sim()?.vehicle(id).map_or_else(Vec::new, |v| v.wheels.iter().map(|w| w.body).collect());
        bodies.into_iter().map(|b| self.body_handle(b)).collect()
    }

    fn rot(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Mat3> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle(id).map_or(ferrosa_physics::rotation::IDENTITY, |v| v.rot).into())
    }

    fn pos(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle_pos(id).unwrap_or(glam::Vec3::ZERO).into())
    }

    fn park_in_traffic(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&vehicle)?.0;
        self.sim_mut()?.park_in_traffic(id);
        Ok(())
    }

    fn delete(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&vehicle)?.0;
        self.sim_mut()?.remove_vehicle(id);
        Ok(())
    }

    fn owner(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Option<u32>> {
        let id = self.table.get(&vehicle)?.0;

        Ok(self.sim()?.vehicle_owner(id).map(|p| p.0))
    }

    fn traffic_car(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Option<u32>> {
        let id = self.table.get(&vehicle)?.0;

        Ok(self.sim()?.vehicle_traffic_car(id).map(|c| c as u32))
    }

    fn set_train_index(&mut self, vehicle: Resource<VehicleRef>, index: i32) -> wasmtime::Result<()> {
        let id = self.table.get(&vehicle)?.0;
        self.sim_mut()?.set_vehicle_train_index(id, index);

        Ok(())
    }

    fn drop(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<()> {
        self.table.delete(vehicle)?;
        Ok(())
    }
}
