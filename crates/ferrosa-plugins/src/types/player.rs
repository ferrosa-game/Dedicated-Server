use ferrosa_sim::{Player, PlayerId};
use wasmtime::component::Resource;

use super::HumanRef;
use crate::{
    bindings::ferrosa::plugin::types::{self, Mat3, Vec3},
    host::HostState,
};

pub struct PlayerRef(pub(crate) PlayerId);

impl HostState {
    fn player_of(&self, player: &Resource<PlayerRef>) -> wasmtime::Result<Option<&Player>> {
        let id = self.table.get(player)?.0;
        Ok(self.sim()?.player(id))
    }

    fn player_id_of(&self, player: &Resource<PlayerRef>) -> wasmtime::Result<PlayerId> {
        Ok(self.table.get(player)?.0)
    }

    pub(crate) fn player_handle(&mut self, id: PlayerId) -> wasmtime::Result<Resource<PlayerRef>> {
        Ok(self.table.push(PlayerRef(id))?)
    }
}

impl types::HostPlayer for HostState {
    fn id(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<u32> {
        Ok(self.player_id_of(&player)?.0)
    }

    fn exists(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        Ok(self.player_of(&player)?.is_some())
    }

    fn name(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<String> {
        Ok(self.player_of(&player)?.map(|p| p.username.clone()).unwrap_or_default())
    }

    fn team(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<types::Team> {
        Ok(self.player_of(&player)?.map_or(types::Team::Spectator, |p| p.team.into()))
    }

    fn admin(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        Ok(self.player_of(&player)?.is_some_and(|p| p.is_admin))
    }

    fn bot(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        Ok(self.player_of(&player)?.is_some_and(|p| p.is_bot))
    }

    fn message(&mut self, player: Resource<PlayerRef>, message: String) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.priv_chat(id, &message);
        Ok(())
    }

    fn human(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Option<Resource<HumanRef>>> {
        let Some(h) = self.player_of(&player)?.and_then(|p| p.human) else { return Ok(None) };
        self.human_handle(h).map(Some)
    }

    fn set_human(&mut self, player: Resource<PlayerRef>, human: Resource<HumanRef>) -> wasmtime::Result<()> {
        let (id, human) = (self.player_id_of(&player)?, self.table.get(&human)?.0);
        self.sim_mut()?.set_player_human(id, human);
        Ok(())
    }

    fn god_mode(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        Ok(self.player_of(&player)?.is_some_and(|p| p.god_mode))
    }

    fn set_god_mode(&mut self, player: Resource<PlayerRef>, on: bool) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_god_mode(id, on);
        Ok(())
    }

    fn menu(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<u8> {
        Ok(self.player_of(&player)?.map_or(0, |p| p.menu as u8))
    }

    fn set_menu(&mut self, player: Resource<PlayerRef>, menu: u8) -> wasmtime::Result<bool> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim_mut()?.set_menu(id, menu))
    }

    fn camera(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Vec3> {
        Ok(self.player_of(&player)?.map_or(glam::Vec3::ZERO, |p| p.camera_pos).into())
    }

    fn yaw(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<f32> {
        Ok(self.player_of(&player)?.map_or(0.0, |p| p.view_yaw))
    }

    fn pitch(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<f32> {
        Ok(self.player_of(&player)?.map_or(0.0, |p| p.view_pitch))
    }

    fn facing(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Mat3> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_facing(id).unwrap_or(ferrosa_physics::rotation::IDENTITY).into())
    }

    fn view(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Mat3> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_view(id).unwrap_or(ferrosa_physics::rotation::IDENTITY).into())
    }

    fn drop(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<()> {
        self.table.delete(player)?;
        Ok(())
    }
}
