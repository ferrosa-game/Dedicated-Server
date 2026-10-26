use ferrosa_sim::PlayerId;
use wasmtime::bail;

use crate::{
    bindings::ferrosa::plugin::{
        server,
        types::{self, ChatType, ClientMode, Intersection, Mat3, Vec3},
    },
    host::{HostState, RegisteredMode},
    types::{BodyRef, BuildingRef, ComputerRef, CorporationRef, HumanRef, ItemRef, PlayerRef, VehicleRef, VolumeRef},
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

        println!("[Plugins] Registered new game mode {name}");
        self.registered = Some(RegisteredMode { name, aliases, client_mode: client_mode.into(), map });
        Ok(())
    }

    fn register_program(&mut self, name: String) -> wasmtime::Result<()> {
        if !self.registering {
            bail!("register-program can only be called from init");
        }

        self.programs.push(name);
        Ok(())
    }

    fn computers(&mut self) -> wasmtime::Result<Vec<Resource<ComputerRef>>> {
        let items = self.sim()?.computer_items();

        items.into_iter().map(|i| self.computer_handle(i)).collect()
    }

    fn get_computer(&mut self, item: u32) -> wasmtime::Result<Option<Resource<ComputerRef>>> {
        let item = item as usize;

        if !self.sim()?.is_computer(item) {
            return Ok(None);
        }

        self.computer_handle(item).map(Some)
    }

    fn new_volume(&mut self, capacity: i32) -> wasmtime::Result<Option<Resource<VolumeRef>>> {
        let Some(volume) = self.sim_mut()?.new_volume(capacity) else { return Ok(None) };

        self.volume_handle(volume).map(Some)
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

    fn add_bot(&mut self) -> wasmtime::Result<Option<Resource<PlayerRef>>> {
        let Some(id) = self.sim_mut()?.add_bot() else { return Ok(None) };
        self.player_handle(id).map(Some)
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

    fn spawn_item(&mut self, name: String, pos: Vec3, rot: Mat3) -> wasmtime::Result<Option<Resource<ItemRef>>> {
        let Some(kind) = self.sim()?.find_item_kind(&name) else {
            eprintln!("[Plugins] Could not spawn {name}");
            return Ok(None);
        };

        self.spawn_item_by_id(kind, pos, rot)
    }

    fn spawn_item_by_id(&mut self, kind: u32, pos: Vec3, rot: Mat3) -> wasmtime::Result<Option<Resource<ItemRef>>> {
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

    fn vehicle_kinds(&mut self) -> wasmtime::Result<Vec<String>> {
        Ok(self.sim()?.vehicle_kind_names())
    }

    fn find_vehicle_kind(&mut self, name: String) -> wasmtime::Result<Option<u32>> {
        Ok(self.sim()?.find_vehicle_kind(&name))
    }

    fn vehicle_display_name(&mut self, kind: u32) -> wasmtime::Result<Option<String>> {
        Ok(self.sim()?.vehicle_display_name(kind).map(str::to_string))
    }

    fn spawn_vehicle(&mut self, name: String, colour: i32, pos: Vec3, rot: Mat3) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let Some(kind) = self.sim()?.find_vehicle_kind(&name) else {
            eprintln!("[Plugins] Could not spawn {name}");
            return Ok(None);
        };

        self.spawn_vehicle_by_id(kind, colour, pos, rot)
    }

    fn spawn_vehicle_by_id(&mut self, kind: u32, colour: i32, pos: Vec3, rot: Mat3) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let Some(id) = self.sim_mut()?.place_vehicle(kind, colour, pos.into(), rot.into()) else { return Ok(None) };
        self.vehicle_handle(id).map(Some)
    }

    fn vehicles(&mut self) -> wasmtime::Result<Vec<Resource<VehicleRef>>> {
        let ids = self.sim()?.vehicle_ids();
        ids.into_iter().map(|id| self.vehicle_handle(id)).collect()
    }

    fn spawn_route_car(&mut self, min_distance: Option<f32>) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        let Some(v) = self.sim_mut()?.spawn_route_car(min_distance) else { return Ok(None) };
        self.vehicle_handle(v).map(Some)
    }

    fn get_vehicle(&mut self, id: u32) -> wasmtime::Result<Option<Resource<VehicleRef>>> {
        if self.sim()?.vehicle(id as usize).is_none() {
            return Ok(None);
        }

        self.vehicle_handle(id as usize).map(Some)
    }

    fn intersections(&mut self) -> wasmtime::Result<Vec<Intersection>> {
        let corners = self.sim()?.intersections().iter().map(|i| Intersection {
            pos: i.world_pos.into(),
            lanes: i.lanes.into(),
            min: i.bounds_min.into(),
            max: i.bounds_max.into(),
        });

        Ok(corners.collect())
    }

    fn buildings(&mut self) -> wasmtime::Result<Vec<Resource<BuildingRef>>> {
        let count = self.sim()?.buildings().len();

        (0..count).map(|id| self.building_handle(id)).collect()
    }

    fn get_building(&mut self, id: u32) -> wasmtime::Result<Option<Resource<BuildingRef>>> {
        if self.sim()?.building(id as usize).is_none() {
            return Ok(None);
        }

        self.building_handle(id as usize).map(Some)
    }

    fn ground_below(&mut self, pos: Vec3) -> wasmtime::Result<Option<f32>> {
        Ok(self.sim()?.ground_below(pos.into()))
    }

    fn clear_bullets(&mut self) -> wasmtime::Result<()> {
        self.sim_mut()?.clear_bullets();
        Ok(())
    }

    fn play_sound(&mut self, sound: types::Sound, pos: types::Vec3, volume: f32, pitch: f32) -> wasmtime::Result<()> {
        self.sim_mut()?.play_sound(sound.into(), pos.into(), volume, pitch);
        Ok(())
    }

    fn get_mode(&mut self) -> wasmtime::Result<String> {
        Ok(self.sim()?.mode_name().to_string())
    }

    fn team_door_open(&mut self, team: types::Team) -> wasmtime::Result<bool> {
        Ok(self.sim()?.team_door_open(team.into()))
    }

    fn set_team_door(&mut self, team: types::Team, open: bool) -> wasmtime::Result<()> {
        self.sim_mut()?.set_team_door_open(team.into(), open);
        Ok(())
    }

    fn get_corporation(&mut self, team: types::Team) -> wasmtime::Result<Option<Resource<CorporationRef>>> {
        let team = team.into();

        if self.sim()?.corporation(team).is_none() {
            return Ok(None);
        }

        self.corporation_handle(team).map(Some)
    }

    fn get_game_timer(&mut self) -> wasmtime::Result<i32> {
        Ok(self.sim()?.game_timer())
    }

    fn set_game_timer(&mut self, ticks: i32) -> wasmtime::Result<()> {
        self.sim_mut()?.set_game_timer(ticks);
        Ok(())
    }

    fn world_time(&mut self) -> wasmtime::Result<i32> {
        Ok(self.sim()?.world_time())
    }

    fn set_world_time(&mut self, time: i32) -> wasmtime::Result<()> {
        self.sim_mut()?.set_world_time(time);
        Ok(())
    }

    fn set_world_time_clock(&mut self, hours: i32, minutes: i32) -> wasmtime::Result<()> {
        self.sim_mut()?.set_world_time_clock(hours, minutes);
        Ok(())
    }

    fn request_reset(&mut self) -> wasmtime::Result<()> {
        self.sim_mut()?.request_reset();
        Ok(())
    }

    fn reset_lobby(&mut self) -> wasmtime::Result<()> {
        self.sim_mut()?.reset_lobby();
        Ok(())
    }

    fn set_week(&mut self, weekly: bool, weekday: u8) -> wasmtime::Result<()> {
        self.sim_mut()?.set_week(weekly, weekday);
        Ok(())
    }

    fn set_traffic(&mut self, cars: u32, parked: bool) -> wasmtime::Result<()> {
        self.sim_mut()?.set_traffic(cars, parked);
        Ok(())
    }

    fn item_price(&mut self, kind: u32) -> wasmtime::Result<Option<i32>> {
        Ok(self.sim()?.item_price(kind))
    }

    fn vehicle_price(&mut self, kind: u32) -> wasmtime::Result<Option<i32>> {
        Ok(self.sim()?.vehicle_price(kind))
    }

    fn get_game_state(&mut self) -> wasmtime::Result<types::GameState> {
        Ok(self.sim()?.gamestate().into())
    }

    fn set_game_state(&mut self, state: types::GameState) -> wasmtime::Result<()> {
        self.sim_mut()?.set_gamestate(state.into());
        Ok(())
    }

    fn set_keep_bodies(&mut self, keep: bool) -> wasmtime::Result<()> {
        self.sim_mut()?.set_keep_bodies(keep);
        Ok(())
    }

    fn set_keep_items(&mut self, keep: bool) -> wasmtime::Result<()> {
        self.sim_mut()?.set_keep_items(keep);
        Ok(())
    }

    fn set_stock_sell_rate(&mut self, rate: f32) -> wasmtime::Result<()> {
        self.sim_mut()?.set_stock_sell_rate(rate);
        Ok(())
    }

    fn set_count_humanless_members(&mut self, on: bool) -> wasmtime::Result<()> {
        self.sim_mut()?.set_count_humanless_members(on);
        Ok(())
    }

    fn set_door_knocks(&mut self, on: bool) -> wasmtime::Result<()> {
        self.sim_mut()?.set_door_knocks(on);
        Ok(())
    }

    fn get_gravity(&mut self) -> wasmtime::Result<f32> {
        Ok(self.sim()?.gravity_scale())
    }

    fn set_gravity(&mut self, scale: f32) -> wasmtime::Result<()> {
        self.sim_mut()?.set_gravity_scale(scale);
        Ok(())
    }

    fn bodies(&mut self) -> wasmtime::Result<Vec<Resource<BodyRef>>> {
        let ids = self.sim()?.body_ids();
        ids.into_iter().map(|id| self.body_handle(id)).collect()
    }

    fn get_body(&mut self, id: u32) -> wasmtime::Result<Option<Resource<BodyRef>>> {
        if self.sim()?.body(id as usize).is_none() {
            return Ok(None);
        }

        self.body_handle(id as usize).map(Some)
    }
}
