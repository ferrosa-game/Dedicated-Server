use ferrosa_physics::RigidBody;
use wasmtime::component::Resource;

use crate::{
    bindings::ferrosa::plugin::types::{self, Vec3},
    host::HostState,
};

pub struct BodyRef(pub(crate) usize);

impl HostState {
    fn body_of(&self, body: &Resource<BodyRef>) -> wasmtime::Result<Option<&RigidBody>> {
        let id = self.table.get(body)?.0;
        Ok(self.sim()?.body(id))
    }

    fn change_body(&mut self, body: &Resource<BodyRef>, f: impl FnOnce(&mut RigidBody)) -> wasmtime::Result<()> {
        let id = self.table.get(body)?.0;

        if let Some(b) = self.sim_mut()?.body_mut(id) {
            f(b);
        }

        Ok(())
    }

    pub(crate) fn body_handle(&mut self, id: usize) -> wasmtime::Result<Resource<BodyRef>> {
        Ok(self.table.push(BodyRef(id))?)
    }
}

impl types::HostBody for HostState {
    fn id(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<u32> {
        Ok(self.table.get(&body)?.0 as u32)
    }

    fn exists(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<bool> {
        Ok(self.body_of(&body)?.is_some())
    }

    fn pos(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<Vec3> {
        Ok(self.body_of(&body)?.map_or(glam::Vec3::ZERO, |b| b.pos).into())
    }

    fn set_pos(&mut self, body: Resource<BodyRef>, pos: Vec3) -> wasmtime::Result<()> {
        self.change_body(&body, |b| b.pos = pos.into())
    }

    fn vel(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<Vec3> {
        Ok(self.body_of(&body)?.map_or(glam::Vec3::ZERO, |b| b.vel).into())
    }

    fn set_vel(&mut self, body: Resource<BodyRef>, vel: Vec3) -> wasmtime::Result<()> {
        self.change_body(&body, |b| b.vel = vel.into())
    }

    fn ang_vel(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<Vec3> {
        Ok(self.body_of(&body)?.map_or(glam::Vec3::ZERO, |b| b.ang_vel).into())
    }

    fn set_ang_vel(&mut self, body: Resource<BodyRef>, ang_vel: Vec3) -> wasmtime::Result<()> {
        self.change_body(&body, |b| b.ang_vel = ang_vel.into())
    }

    fn mass(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<f32> {
        Ok(self.body_of(&body)?.map_or(0.0, |b| b.mass))
    }

    fn settled(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<bool> {
        Ok(self.body_of(&body)?.is_some_and(|b| b.settled))
    }

    fn add_impulse(&mut self, body: Resource<BodyRef>, offset: Vec3, impulse: Vec3) -> wasmtime::Result<()> {
        let id = self.table.get(&body)?.0;
        self.sim_mut()?.add_impulse(id, offset.into(), impulse.into());
        Ok(())
    }

    fn drop(&mut self, body: Resource<BodyRef>) -> wasmtime::Result<()> {
        self.table.delete(body)?;
        Ok(())
    }
}
