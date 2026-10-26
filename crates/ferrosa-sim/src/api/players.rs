use ferrosa_physics::{
    RotMatrix,
    rotation::{IDENTITY, rotate_orientation},
};
use ferrosa_protocol::{
    CharacterCustomization, Team,
    clientbound::game::{
        MenuType,
        events::{Event, ServerEvent, mission::EventMission},
    },
};
use glam::Vec3;

use super::mark;
use crate::{PlayerId, player::Player, sim::Sim};

impl Sim {
    pub fn player_ids(&self) -> Vec<PlayerId> {
        self.players.iter().map(|(_, p)| p.player_id).collect()
    }

    pub fn player(&self, player: PlayerId) -> Option<&Player> {
        self.players.get(player.idx())
    }

    pub fn player_view(&self, player: PlayerId) -> Option<RotMatrix> {
        let mut view = self.player_facing(player)?;
        let right = -view[2];
        rotate_orientation(&mut view, right, self.players.get(player.idx())?.view_pitch);

        Some(view)
    }

    pub fn player_facing(&self, player: PlayerId) -> Option<RotMatrix> {
        let p = self.players.get(player.idx())?;
        let mut facing = IDENTITY;
        rotate_orientation(&mut facing, Vec3::Y, p.view_yaw);

        Some(facing)
    }

    pub fn set_god_mode(&mut self, player: PlayerId, on: bool) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.god_mode = on;
        }
    }

    pub fn set_menu(&mut self, player: PlayerId, menu: MenuType) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.menu = menu;
        }
    }

    pub fn set_team_switch_timer(&mut self, player: PlayerId, ticks: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.team_switch_timer = ticks;
        }
    }

    pub fn set_team(&mut self, player: PlayerId, team: Team) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.team = team;

        mark(&mut self.updates.players, player);
    }

    pub fn send_mission_event(&mut self, player: PlayerId, kind: i32, role: i32, value: i32, pos: Option<Vec3>) {
        let e = EventMission { player: player.0 as i32, kind, role, value, pos: pos.map_or([0.0; 3], |p| p.to_array()) };
        self.events.push(Event { tick_created: self.tick, kind: ServerEvent::Mission(e) });
    }

    pub fn set_name(&mut self, player: PlayerId, name: &str) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.username = name.to_string();

        mark(&mut self.updates.players, player);
    }

    pub fn set_ready(&mut self, player: PlayerId, ready: bool) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.is_ready = ready;

        mark(&mut self.updates.players, player);
    }

    pub fn set_manager_tab(&mut self, player: PlayerId, on: bool) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.manager_tab = on;

        mark(&mut self.updates.players, player);
    }

    pub fn set_menu_tab(&mut self, player: PlayerId, tab: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.menu_tab = tab;
        }
    }

    pub fn set_items_bought(&mut self, player: PlayerId, count: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.items_bought = count;
        }
    }

    pub fn set_customization(&mut self, player: PlayerId, look: CharacterCustomization) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.customization = look;

        if let Some(h) = p.human.and_then(|h| self.humans.get_mut(h)) {
            h.customization.model = look.model;
            h.customization.suit_color = look.suit_color;
            h.customization.tie_color = look.tie_color;
            h.customization.necklace = look.necklace;
        }

        mark(&mut self.updates.players, player);
    }

    pub fn add_menu_button(&mut self, player: PlayerId, id: i32, text: &str, extra: Option<i32>) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        let end = text.floor_char_boundary(63);

        p.push_button(id, &text[..end], extra);
    }

    pub fn set_money(&mut self, player: PlayerId, money: i32) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.money = money;

        mark(&mut self.updates.rounds, player);
    }

    pub fn set_player_stocks(&mut self, player: PlayerId, stocks: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.stocks = stocks;
        }

        self.push_round_update(player);
    }

    pub fn buy_player_stocks(&mut self, player: PlayerId, count: i32) {
        self.purchase_stocks(player, count);
        self.push_round_update(player);
    }

    pub fn sell_player_stocks(&mut self, player: PlayerId, count: i32) {
        self.sell_stocks(player, count);
        self.push_round_update(player);
    }

    pub fn sell_all_player_stocks(&mut self, player: PlayerId) {
        self.sell_all_stocks(player);
        self.push_round_update(player);
    }

    fn push_round_update(&mut self, player: PlayerId) {
        mark(&mut self.updates.rounds, player);
    }

    pub fn add_bot(&mut self) -> Option<PlayerId> {
        let pid = self.create_player()?;
        mark(&mut self.updates.players, pid);
        Some(pid)
    }

    pub fn remove_bot(&mut self, player: PlayerId) -> bool {
        if !self.players.get(player.idx()).is_some_and(|p| p.is_bot) {
            return false;
        }

        self.delete_bot_player(player);
        true
    }

    pub fn set_bot_waypoints(&mut self, player: PlayerId, waypoints: Vec<Vec3>) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.bot.waypoint = 0;
        p.bot.waypoint_count = waypoints.len() as i32;
        p.bot.waypoints = waypoints;
    }

    pub fn set_player_human(&mut self, player: PlayerId, human: usize) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.human = Some(human);
        p.ghost_human = false;
        p.items_bought = 0;

        mark(&mut self.updates.players, player);
    }
}
