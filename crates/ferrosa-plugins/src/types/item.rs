use wasmtime::component::Resource;

use crate::{
    bindings::ferrosa::plugin::types::{self, Vec3},
    host::HostState,
};

pub struct ItemRef(pub(crate) usize);

impl HostState {
    pub(crate) fn item_handle(&mut self, id: usize) -> wasmtime::Result<Resource<ItemRef>> {
        Ok(self.table.push(ItemRef(id))?)
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
