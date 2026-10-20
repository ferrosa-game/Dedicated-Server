use ferrosa_sim::{ModeRules, Sim};
use wasmtime::bail;

use crate::{
    bindings::ferrosa::plugin::{
        server,
        types::{self, Body, ClientMode, Rules, Vec3},
    },
    host::{HostState, RegisteredMode},
};

impl server::Host for HostState {
    fn register_mode(&mut self, name: String, aliases: Vec<String>, client_mode: ClientMode, map: Option<String>) -> wasmtime::Result<()> {
        if !self.registering {
            bail!("register-mode can only be called from init");
        }

        if self.registered.is_some() {
            bail!("a plugin can only register one mode");
        }

        self.registered = Some(RegisteredMode { name, aliases, client_mode: client_mode.into(), map });
        Ok(())
    }

    fn get_game_state(&mut self) -> wasmtime::Result<types::GameState> {
        Ok(self.sim()?.gamestate().into())
    }

    fn set_game_state(&mut self, state: types::GameState) -> wasmtime::Result<()> {
        self.sim_mut()?.set_gamestate(state.into());
        Ok(())
    }

    fn default_rules(&mut self) -> wasmtime::Result<Rules> {
        Ok(ModeRules::default().into())
    }

    fn get_gravity(&mut self) -> wasmtime::Result<f32> {
        Ok(self.sim()?.gravity_scale())
    }

    fn set_gravity(&mut self, scale: f32) -> wasmtime::Result<()> {
        self.sim_mut()?.set_gravity_scale(scale);
        Ok(())
    }

    fn bodies(&mut self) -> wasmtime::Result<Vec<Body>> {
        let sim = self.sim()?;
        Ok(sim.body_ids().into_iter().filter_map(|id| body(sim, id)).collect())
    }

    fn get_body(&mut self, id: u32) -> wasmtime::Result<Option<Body>> {
        Ok(body(self.sim()?, id as usize))
    }

    fn set_body_pos(&mut self, id: u32, pos: Vec3) -> wasmtime::Result<()> {
        if let Some(b) = self.sim_mut()?.body_mut(id as usize) {
            b.pos = pos.into();
        }

        Ok(())
    }

    fn set_body_vel(&mut self, id: u32, vel: Vec3) -> wasmtime::Result<()> {
        if let Some(b) = self.sim_mut()?.body_mut(id as usize) {
            b.vel = vel.into();
        }

        Ok(())
    }

    fn set_body_ang_vel(&mut self, id: u32, ang_vel: Vec3) -> wasmtime::Result<()> {
        if let Some(b) = self.sim_mut()?.body_mut(id as usize) {
            b.ang_vel = ang_vel.into();
        }

        Ok(())
    }

    fn add_impulse(&mut self, id: u32, offset: Vec3, impulse: Vec3) -> wasmtime::Result<()> {
        self.sim_mut()?.add_impulse(id as usize, offset.into(), impulse.into());
        Ok(())
    }
}

fn body(sim: &Sim, id: usize) -> Option<Body> {
    let b = sim.body(id)?;

    Some(Body { id: id as u32, pos: b.pos.into(), vel: b.vel.into(), ang_vel: b.ang_vel.into(), mass: b.mass, settled: b.settled })
}
