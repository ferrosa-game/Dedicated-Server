use wasmtime::component::Resource;

use super::{BodyRef, ComputerRef, HumanRef, VehicleRef, VolumeRef};
use crate::{
    bindings::ferrosa::plugin::types::{self, Cash, Vec3},
    host::HostState,
};

pub struct ItemRef(pub(crate) usize);

impl HostState {
    pub(crate) fn item_handle(&mut self, id: usize) -> wasmtime::Result<Resource<ItemRef>> {
        Ok(self.table.push(ItemRef(id))?)
    }
}

/// A note's code on the wire, which is its place in the list.
fn code(b: types::Bill) -> u32 {
    b as u32
}

/// The note a code names; the eighth is worth nothing.
fn bill(code: u32) -> types::Bill {
    match code {
        0 => types::Bill::One,
        1 => types::Bill::Five,
        2 => types::Bill::Ten,
        3 => types::Bill::Twenty,
        4 => types::Bill::Fifty,
        5 => types::Bill::Hundred,
        6 => types::Bill::Thousand,
        _ => types::Bill::None,
    }
}

impl types::HostItem for HostState {
    fn id(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<u32> {
        Ok(self.table.get(&item)?.0 as u32)
    }

    fn exists(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item(id).is_some())
    }

    fn kind(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<u32> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item(id).map_or(0, |i| i.item_type as u32))
    }

    fn pos(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Vec3> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item(id).map_or(glam::Vec3::ZERO, |i| i.pos).into())
    }

    fn phone_number(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<i32>> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.phone_number(id))
    }

    fn set_phone_number(&mut self, item: Resource<ItemRef>, number: i32) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim_mut()?.set_phone_number(id, number))
    }

    fn set_radio_channel(&mut self, item: Resource<ItemRef>, channel: i32) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim_mut()?.set_radio_channel(id, channel))
    }

    fn set_phone_texture(&mut self, item: Resource<ItemRef>, texture: i32) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim_mut()?.set_phone_texture(id, texture))
    }

    fn set_key_vehicle(&mut self, item: Resource<ItemRef>, vehicle: Option<Resource<VehicleRef>>) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        let vehicle = vehicle.map(|v| self.table.get(&v).map(|v| v.0)).transpose()?;
        Ok(self.sim_mut()?.set_key_vehicle(id, vehicle))
    }

    fn attach(&mut self, item: Resource<ItemRef>, child: Resource<ItemRef>) -> wasmtime::Result<bool> {
        let (id, child) = (self.table.get(&item)?.0, self.table.get(&child)?.0);
        Ok(self.sim_mut()?.attach_item(id, child))
    }

    fn holder(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<Resource<HumanRef>>> {
        let id = self.table.get(&item)?.0;
        let Some(h) = self.sim()?.item_holder(id) else { return Ok(None) };
        self.human_handle(h).map(Some)
    }

    fn set_volume(&mut self, item: Resource<ItemRef>, volume: Resource<VolumeRef>) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        let volume = self.table.get(&volume)?.0;

        Ok(self.sim_mut()?.set_item_volume(id, volume))
    }

    fn computer(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<Resource<ComputerRef>>> {
        let id = self.table.get(&item)?.0;

        if !self.sim()?.is_computer(id) {
            return Ok(None);
        }

        self.computer_handle(id).map(Some)
    }

    fn volume(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<Resource<VolumeRef>>> {
        let id = self.table.get(&item)?.0;
        let Some(volume) = self.sim()?.item_volume(id) else { return Ok(None) };

        self.volume_handle(volume).map(Some)
    }

    fn children(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Vec<Resource<ItemRef>>> {
        let id = self.table.get(&item)?.0;
        let children = self.sim()?.item_children(id);
        children.into_iter().map(|c| self.item_handle(c)).collect()
    }

    fn body(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<Resource<BodyRef>>> {
        let id = self.table.get(&item)?.0;
        let Some(body) = self.sim()?.item(id).map(|i| i.body) else { return Ok(None) };
        self.body_handle(body).map(Some)
    }

    fn is_static(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item_is_static(id))
    }

    fn set_is_static(&mut self, item: Resource<ItemRef>, still: bool) -> wasmtime::Result<()> {
        let id = self.table.get(&item)?.0;
        self.sim_mut()?.set_item_is_static(id, still);

        Ok(())
    }

    fn despawn_time(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item_despawn_time(id))
    }

    fn set_despawn_time(&mut self, item: Resource<ItemRef>, ticks: i32) -> wasmtime::Result<()> {
        let id = self.table.get(&item)?.0;
        self.sim_mut()?.set_item_despawn_time(id, ticks);

        Ok(())
    }

    fn write_memo(&mut self, item: Resource<ItemRef>, text: String) -> wasmtime::Result<()> {
        let id = self.table.get(&item)?.0;
        self.sim_mut()?.write_item_memo(id, &text);

        Ok(())
    }

    fn uses(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<i32> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item_uses(id))
    }

    fn set_uses(&mut self, item: Resource<ItemRef>, uses: i32) -> wasmtime::Result<()> {
        let id = self.table.get(&item)?.0;
        self.sim_mut()?.set_item_uses(id, uses);
        Ok(())
    }

    fn key_vehicle(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let id = self.table.get(&item)?.0;
        let Some(v) = self.sim()?.key_vehicle(id) else { return Ok(None) };
        self.vehicle_handle(v).map(Some)
    }

    fn cash(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<Cash>> {
        let id = self.table.get(&item)?.0;

        Ok(self.sim()?.item_cash(id).map(|(top, codes)| Cash { bills: (0..=top).map(|k| bill((codes >> (k * 3)) & 7)).collect() }))
    }

    fn set_cash(&mut self, item: Resource<ItemRef>, cash: Cash) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        let codes = cash.bills.iter().enumerate().fold(0u32, |v, (k, &b)| v | (code(b) << (k * 3)));

        Ok(self.sim_mut()?.set_item_cash(id, cash.bills.len() as i32 - 1, codes))
    }

    fn cash_value(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<Option<i32>> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim()?.item_cash_value(id))
    }

    fn insert_bill(&mut self, item: Resource<ItemRef>, at: u32, b: types::Bill) -> wasmtime::Result<bool> {
        let id = self.table.get(&item)?.0;
        Ok(self.sim_mut()?.insert_item_bill(id, at as i32, code(b)))
    }

    fn delete(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<()> {
        let id = self.table.get(&item)?.0;
        self.sim_mut()?.remove_item(id);
        Ok(())
    }

    fn drop(&mut self, item: Resource<ItemRef>) -> wasmtime::Result<()> {
        self.table.delete(item)?;
        Ok(())
    }
}
