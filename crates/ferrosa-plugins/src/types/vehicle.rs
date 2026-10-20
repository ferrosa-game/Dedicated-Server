use wasmtime::component::Resource;

use crate::{
    bindings::ferrosa::plugin::types::{self, Vec3},
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

    fn pos(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&vehicle)?.0;
        Ok(self.sim()?.vehicle_pos(id).unwrap_or(glam::Vec3::ZERO).into())
    }

    fn delete(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&vehicle)?.0;
        self.sim_mut()?.remove_vehicle(id);
        Ok(())
    }

    fn drop(&mut self, vehicle: Resource<VehicleRef>) -> wasmtime::Result<()> {
        self.table.delete(vehicle)?;
        Ok(())
    }
}
