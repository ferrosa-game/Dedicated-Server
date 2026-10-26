use ferrosa_protocol::{CharacterCustomization, Gender, Model, SuitColor};
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

    fn change_look(&mut self, player: &Resource<PlayerRef>, f: impl FnOnce(&mut CharacterCustomization)) -> wasmtime::Result<()> {
        let id = self.player_id_of(player)?;
        let Some(mut look) = self.player_of(player)?.map(|p| p.customization) else { return Ok(()) };
        f(&mut look);

        self.sim_mut()?.set_customization(id, look);
        Ok(())
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

    fn set_name(&mut self, player: Resource<PlayerRef>, name: String) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_name(id, &name);
        Ok(())
    }

    fn gender(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<types::Gender> {
        Ok(self.player_of(&player)?.map_or(Gender::default(), |p| p.customization.gender).into())
    }

    fn set_gender(&mut self, player: Resource<PlayerRef>, gender: types::Gender) -> wasmtime::Result<()> {
        self.change_look(&player, |l| l.gender = gender.into())
    }

    fn team(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<types::Team> {
        Ok(self.player_of(&player)?.map_or(types::Team::Spectator, |p| p.team.into()))
    }

    fn set_team(&mut self, player: Resource<PlayerRef>, team: types::Team) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_team(id, team.into());
        Ok(())
    }

    fn ready(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        Ok(self.player_of(&player)?.is_some_and(|p| p.is_ready))
    }

    fn set_ready(&mut self, player: Resource<PlayerRef>, ready: bool) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_ready(id, ready);
        Ok(())
    }

    fn manager_tab(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        Ok(self.player_of(&player)?.is_some_and(|p| p.manager_tab))
    }

    fn set_manager_tab(&mut self, player: Resource<PlayerRef>, on: bool) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_manager_tab(id, on);
        Ok(())
    }

    fn menu_tab(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        Ok(self.player_of(&player)?.map_or(-1, |p| p.menu_tab))
    }

    fn set_menu_tab(&mut self, player: Resource<PlayerRef>, tab: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_menu_tab(id, tab);
        Ok(())
    }

    fn items_bought(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        Ok(self.player_of(&player)?.map_or(0, |p| p.items_bought))
    }

    fn set_items_bought(&mut self, player: Resource<PlayerRef>, count: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_items_bought(id, count);
        Ok(())
    }

    fn account_id(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Option<u32>> {
        Ok(self.player_of(&player)?.filter(|p| !p.is_bot).map(|p| p.account_id))
    }

    fn model(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<types::Model> {
        Ok(self.player_of(&player)?.map_or(Model::default(), |p| p.customization.model).into())
    }

    fn set_model(&mut self, player: Resource<PlayerRef>, model: types::Model) -> wasmtime::Result<()> {
        self.change_look(&player, |l| l.model = model.into())
    }

    fn suit_colour(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<types::SuitColour> {
        Ok(self.player_of(&player)?.map_or(SuitColor::default(), |p| p.customization.suit_color).into())
    }

    fn set_suit_colour(&mut self, player: Resource<PlayerRef>, colour: types::SuitColour) -> wasmtime::Result<()> {
        self.change_look(&player, |l| l.suit_color = colour.into())
    }

    fn tie_colour(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Option<types::SuitColour>> {
        let tie = self.player_of(&player)?.map_or(0, |p| p.customization.tie_color);

        Ok(tie.checked_sub(1).and_then(|c| SuitColor::try_from(c).ok()).map(Into::into))
    }

    fn set_tie_colour(&mut self, player: Resource<PlayerRef>, colour: Option<types::SuitColour>) -> wasmtime::Result<()> {
        let tie = colour.map_or(0, |c| SuitColor::from(c) as u8 + 1);

        self.change_look(&player, |l| l.tie_color = tie)
    }

    fn necklace(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<u8> {
        Ok(self.player_of(&player)?.map_or(0, |p| p.customization.necklace))
    }

    fn set_necklace(&mut self, player: Resource<PlayerRef>, necklace: u8) -> wasmtime::Result<()> {
        self.change_look(&player, |l| l.necklace = necklace)
    }

    fn add_menu_button(&mut self, player: Resource<PlayerRef>, id: i32, text: String, extra: Option<i32>) -> wasmtime::Result<()> {
        let pid = self.player_id_of(&player)?;
        self.sim_mut()?.add_menu_button(pid, id, &text, extra);
        Ok(())
    }

    fn set_corp_money(&mut self, player: Resource<PlayerRef>, money: i32, credit: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_corp_money(id, money, credit);

        Ok(())
    }

    fn corp_rating(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_corp_rating(id))
    }

    fn set_corp_rating(&mut self, player: Resource<PlayerRef>, rating: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_corp_rating(id, rating);

        Ok(())
    }

    fn spawn_timer(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_spawn_timer(id))
    }

    fn set_spawn_timer(&mut self, player: Resource<PlayerRef>, ticks: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_spawn_timer(id, ticks);

        Ok(())
    }

    fn vehicles_bought(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_vehicles_bought(id))
    }

    fn set_vehicles_bought(&mut self, player: Resource<PlayerRef>, count: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_vehicles_bought(id, count);

        Ok(())
    }

    fn crim_rating(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_crim_rating(id))
    }

    fn set_crim_rating(&mut self, player: Resource<PlayerRef>, rating: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_crim_rating(id, rating);

        Ok(())
    }

    fn bot_deadline(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim()?.player_bot_deadline(id))
    }

    fn set_bot_deadline(&mut self, player: Resource<PlayerRef>, clock: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_bot_deadline(id, clock);

        Ok(())
    }

    fn team_switch_timer(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        Ok(self.player_of(&player)?.map_or(0, |p| p.team_switch_timer))
    }

    fn set_team_switch_timer(&mut self, player: Resource<PlayerRef>, ticks: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_team_switch_timer(id, ticks);
        Ok(())
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

    fn send_mission_event(&mut self, player: Resource<PlayerRef>, kind: i32, role: i32, value: i32, pos: Option<Vec3>) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.send_mission_event(id, kind, role, value, pos.map(Into::into));
        Ok(())
    }

    fn human(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<Option<Resource<HumanRef>>> {
        let Some(h) = self.player_of(&player)?.and_then(|p| p.human) else { return Ok(None) };
        self.human_handle(h).map(Some)
    }

    fn money(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        Ok(self.player_of(&player)?.map_or(0, |p| p.money))
    }

    fn set_money(&mut self, player: Resource<PlayerRef>, money: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_money(id, money);
        Ok(())
    }

    fn stocks(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<i32> {
        Ok(self.player_of(&player)?.map_or(0, |p| p.stocks))
    }

    fn set_stocks(&mut self, player: Resource<PlayerRef>, stocks: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_player_stocks(id, stocks);
        Ok(())
    }

    fn buy_stocks(&mut self, player: Resource<PlayerRef>, count: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.buy_player_stocks(id, count);
        Ok(())
    }

    fn sell_stocks(&mut self, player: Resource<PlayerRef>, count: i32) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.sell_player_stocks(id, count);
        Ok(())
    }

    fn sell_all_stocks(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.sell_all_player_stocks(id);
        Ok(())
    }

    fn remove(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<bool> {
        let id = self.player_id_of(&player)?;
        Ok(self.sim_mut()?.remove_bot(id))
    }

    fn set_waypoints(&mut self, player: Resource<PlayerRef>, waypoints: Vec<Vec3>) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_bot_waypoints(id, waypoints.into_iter().map(Into::into).collect());
        Ok(())
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

    fn menu(&mut self, player: Resource<PlayerRef>) -> wasmtime::Result<types::Menu> {
        Ok(self.player_of(&player)?.map_or(types::Menu::Empty, |p| p.menu.into()))
    }

    fn set_menu(&mut self, player: Resource<PlayerRef>, menu: types::Menu) -> wasmtime::Result<()> {
        let id = self.player_id_of(&player)?;
        self.sim_mut()?.set_menu(id, menu.into());
        Ok(())
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
