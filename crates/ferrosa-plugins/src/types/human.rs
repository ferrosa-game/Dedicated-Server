use ferrosa_sim::Human;
use wasmtime::component::Resource;

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
        Ok(self.human_of(&human)?.map_or(Vec3 { x: 0.0, y: 0.0, z: 0.0 }, |h| h.pos.into()))
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
