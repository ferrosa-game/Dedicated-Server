use ferrosa_sim::Human;
use wasmtime::component::Resource;

use super::{BodyRef, ItemRef, VehicleRef};
use crate::{
    bindings::ferrosa::plugin::types::{self, Vec3},
    host::HostState,
};

pub struct HumanRef(pub(crate) usize);

impl HostState {
    fn human_of(&self, human: &Resource<HumanRef>) -> wasmtime::Result<Option<&Human>> {
        let id = self.table.get(human)?.0;
        Ok(self.sim()?.human(id))
    }

    fn human_mut_of(&mut self, human: &Resource<HumanRef>) -> wasmtime::Result<Option<&mut Human>> {
        let id = self.table.get(human)?.0;
        Ok(self.sim_mut()?.human_mut(id))
    }

    pub(crate) fn human_handle(&mut self, id: usize) -> wasmtime::Result<Resource<HumanRef>> {
        Ok(self.table.push(HumanRef(id))?)
    }
}

impl types::HostHuman for HostState {
    fn id(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<u32> {
        Ok(self.table.get(&human)?.0 as u32)
    }

    fn exists(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<bool> {
        Ok(self.human_of(&human)?.is_some())
    }

    fn pos(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<Vec3> {
        Ok(self.human_of(&human)?.and_then(|h| h.bones.first()).map_or(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, |b| b.pos.into()))
    }

    fn vitality(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<i32> {
        Ok(self.human_of(&human)?.map_or(0, |h| h.health.vitality))
    }

    fn set_vitality(&mut self, human: Resource<HumanRef>, vitality: i32) -> wasmtime::Result<()> {
        if let Some(h) = self.human_mut_of(&human)? {
            h.health.vitality = vitality;
        }

        Ok(())
    }

    fn bleeding(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<bool> {
        Ok(self.human_of(&human)?.is_some_and(|h| h.bleeding))
    }

    fn set_bleeding(&mut self, human: Resource<HumanRef>, bleeding: bool) -> wasmtime::Result<()> {
        if let Some(h) = self.human_mut_of(&human)? {
            h.bleeding = bleeding;
        }

        Ok(())
    }

    fn player(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<Option<u32>> {
        Ok(self.human_of(&human)?.and_then(|h| h.player).map(|p| p.0))
    }

    fn health(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<types::Health> {
        Ok(self.human_of(&human)?.map_or_else(Default::default, |h| h.health).into())
    }

    fn set_health(&mut self, human: Resource<HumanRef>, health: types::Health) -> wasmtime::Result<()> {
        if let Some(h) = self.human_mut_of(&human)? {
            h.health = health.into();
        }

        Ok(())
    }

    fn damage(&mut self, human: Resource<HumanRef>, bone: types::Bone, damage: i32) -> wasmtime::Result<()> {
        let id = self.table.get(&human)?.0;
        self.sim_mut()?.damage_human(id, bone.into(), damage);
        Ok(())
    }

    fn vehicle(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let Some(v) = self.human_of(&human)?.and_then(|h| h.vehicle) else { return Ok(None) };
        self.vehicle_handle(v).map(Some)
    }

    fn last_vehicle(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let Some(v) = self.human_of(&human)?.and_then(|h| usize::try_from(h.last_vehicle).ok()) else { return Ok(None) };
        self.vehicle_handle(v).map(Some)
    }

    fn bones(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<Vec<Resource<BodyRef>>> {
        let bodies: Vec<usize> = self.human_of(&human)?.map_or_else(Vec::new, |h| h.bones.iter().map(|b| b.body).collect());
        bodies.into_iter().map(|b| self.body_handle(b)).collect()
    }

    fn enter_vehicle(&mut self, human: Resource<HumanRef>, vehicle: Resource<VehicleRef>, seat: u32) -> wasmtime::Result<bool> {
        let (id, vehicle) = (self.table.get(&human)?.0, self.table.get(&vehicle)?.0);
        Ok(self.sim_mut()?.enter_vehicle(id, vehicle, seat as usize))
    }

    fn set_view_yaw(&mut self, human: Resource<HumanRef>, yaw: f32) -> wasmtime::Result<()> {
        let id = self.table.get(&human)?.0;
        self.sim_mut()?.set_view_yaw(id, yaw);
        Ok(())
    }

    fn give_item(&mut self, human: Resource<HumanRef>, kind: u32, first: u32, end: u32) -> wasmtime::Result<Option<Resource<ItemRef>>> {
        let id = self.table.get(&human)?.0;
        let slots = first.min(end) as usize..end as usize;
        let Some(item) = self.sim_mut()?.give_human_item(id, kind, slots) else { return Ok(None) };
        self.item_handle(item).map(Some)
    }

    fn slot(&mut self, human: Resource<HumanRef>, slot: u32) -> wasmtime::Result<Vec<Resource<ItemRef>>> {
        let id = self.table.get(&human)?.0;
        let items = self.sim()?.human_slot_items(id, slot as usize);
        items.into_iter().map(|i| self.item_handle(i)).collect()
    }

    fn put_item(&mut self, human: Resource<HumanRef>, item: Resource<ItemRef>, slot: u32) -> wasmtime::Result<()> {
        let (id, item) = (self.table.get(&human)?.0, self.table.get(&item)?.0);
        self.sim_mut()?.put_item(id, item, slot as usize);
        Ok(())
    }

    fn vehicle_seat(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<Option<u32>> {
        let id = self.table.get(&human)?.0;
        Ok(self.sim()?.human_vehicle_seat(id))
    }

    fn yaw(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<f32> {
        let id = self.table.get(&human)?.0;
        Ok(self.sim()?.human_yaw(id))
    }

    fn max_stamina(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&human)?.0;
        Ok(self.sim()?.human_max_stamina(id))
    }

    fn set_max_stamina(&mut self, human: Resource<HumanRef>, stamina: i32) -> wasmtime::Result<()> {
        let id = self.table.get(&human)?.0;
        self.sim_mut()?.set_human_max_stamina(id, stamina);

        Ok(())
    }

    fn release_from_bot(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&human)?.0;
        self.sim_mut()?.release_bot_human(id);

        Ok(())
    }

    fn delete(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&human)?.0;
        self.sim_mut()?.remove_human(id);
        Ok(())
    }

    fn drop(&mut self, human: Resource<HumanRef>) -> wasmtime::Result<()> {
        self.table.delete(human)?;
        Ok(())
    }
}
