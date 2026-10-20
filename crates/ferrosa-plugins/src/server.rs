use ferrosa_sim::{ModeRules, PlayerId, Sim};
use wasmtime::bail;

use crate::{
    bindings::ferrosa::plugin::{
        server,
        types::{self, Body, ChatType, ClientMode, KindName, Mat3, Rules, Vec3},
    },
    host::{HostState, RegisteredMode},
    types::{HumanRef, ItemRef, PlayerRef, VehicleRef},
};
use wasmtime::component::Resource;

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

    fn announce(&mut self, message: String) -> wasmtime::Result<()> {
        self.sim_mut()?.announce(&message);
        Ok(())
    }

    fn send_chat(&mut self, kind: ChatType, speaker: i32, volume: i32, message: String) -> wasmtime::Result<()> {
        self.sim_mut()?.chat(kind.into(), speaker, volume, &message);
        Ok(())
    }

    fn get_player(&mut self, id: u32) -> wasmtime::Result<Option<Resource<PlayerRef>>> {
        if self.sim()?.player(PlayerId(id)).is_none() {
            return Ok(None);
        }

        self.player_handle(PlayerId(id)).map(Some)
    }

    fn players(&mut self) -> wasmtime::Result<Vec<Resource<PlayerRef>>> {
        let ids = self.sim()?.player_ids();
        ids.into_iter().map(|id| self.player_handle(id)).collect()
    }

    fn spawn_human(&mut self, pos: Vec3, rot: Mat3, player: Option<u32>) -> wasmtime::Result<Option<Resource<HumanRef>>> {
        let Some(id) = self.sim_mut()?.place_human(pos.into(), rot.into(), player.map(PlayerId)) else { return Ok(None) };
        self.human_handle(id).map(Some)
    }

    fn humans(&mut self) -> wasmtime::Result<Vec<Resource<HumanRef>>> {
        let ids = self.sim()?.human_ids();
        ids.into_iter().map(|id| self.human_handle(id)).collect()
    }

    fn get_human(&mut self, id: u32) -> wasmtime::Result<Option<Resource<HumanRef>>> {
        if self.sim()?.human(id as usize).is_none() {
            return Ok(None);
        }

        self.human_handle(id as usize).map(Some)
    }

    fn item_kinds(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(self.sim()?.item_kind_names())
    }

    fn find_item_kind(&mut self, name: String) -> wasmtime::Result<Option<u32>> {
        Ok(self.sim()?.find_item_kind(&name))
    }

    fn spawn_item(&mut self, kind: u32, pos: Vec3, rot: Mat3) -> wasmtime::Result<Option<Resource<ItemRef>>> {
        let Some(id) = self.sim_mut()?.spawn_item(kind, pos.into(), rot.into()) else { return Ok(None) };
        self.item_handle(id).map(Some)
    }

    fn items(&mut self) -> wasmtime::Result<Vec<Resource<ItemRef>>> {
        let ids = self.sim()?.item_ids();
        ids.into_iter().map(|id| self.item_handle(id)).collect()
    }

    fn get_item(&mut self, id: u32) -> wasmtime::Result<Option<Resource<ItemRef>>> {
        if self.sim()?.item(id as usize).is_none() {
            return Ok(None);
        }

        self.item_handle(id as usize).map(Some)
    }

    fn vehicle_kinds(&mut self) -> wasmtime::Result<Vec<KindName>> {
        Ok(self.sim()?.vehicle_kind_names().into_iter().map(|(id, name)| KindName { id, name }).collect())
    }

    fn spawn_vehicle(&mut self, kind: u32, colour: i32, pos: Vec3, rot: Mat3) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let Some(id) = self.sim_mut()?.place_vehicle(kind, colour, pos.into(), rot.into()) else { return Ok(None) };
        self.vehicle_handle(id).map(Some)
    }

    fn vehicles(&mut self) -> wasmtime::Result<Vec<Resource<VehicleRef>>> {
        let ids = self.sim()?.vehicle_ids();
        ids.into_iter().map(|id| self.vehicle_handle(id)).collect()
    }

    fn get_vehicle(&mut self, id: u32) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        if self.sim()?.vehicle(id as usize).is_none() {
            return Ok(None);
        }

        self.vehicle_handle(id as usize).map(Some)
    }

    fn ground_below(&mut self, pos: Vec3) -> wasmtime::Result<Option<f32>> {
        Ok(self.sim()?.ground_below(pos.into()))
    }

    fn clear_bullets(&mut self) -> wasmtime::Result<()> {
        self.sim_mut()?.clear_bullets();
        Ok(())
    }

    fn get_mode(&mut self) -> wasmtime::Result<String> {
        Ok(self.sim()?.mode_name().to_string())
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
