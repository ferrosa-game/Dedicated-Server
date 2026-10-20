use ferrosa_physics::{
    RigidBody, RotMatrix,
    rotation::{IDENTITY, rotate, rotate_orientation},
};
use ferrosa_protocol::clientbound::game::{GameState, ItemKind, MenuType, VehicleKind, events::chat::ChatType};
use glam::Vec3;

use crate::{PlayerId, human::Human, items::Item, player::Player, sim::Sim, vehicle::Vehicle, world::trace::line_intersect_level};

fn chat_lines(message: &str) -> impl Iterator<Item = &str> {
    message.lines().flat_map(|line| {
        let mut rest = line;

        std::iter::from_fn(move || {
            if rest.is_empty() {
                return None;
            }

            if rest.len() <= 63 {
                return Some(std::mem::take(&mut rest));
            }

            let limit = rest.floor_char_boundary(63);
            let cut = if rest[limit..].starts_with(' ') { limit } else { rest[..limit].rfind(' ').filter(|&k| k > 0).unwrap_or(limit) };
            let (head, tail) = rest.split_at(cut);
            rest = tail.trim_start();
            Some(head)
        })
    })
}

impl Sim {
    pub fn gamestate(&self) -> GameState {
        self.gamestate
    }

    pub fn set_gamestate(&mut self, state: GameState) {
        self.gamestate = state;
    }

    pub fn announce(&mut self, message: &str) {
        for line in chat_lines(message) {
            self.chat(ChatType::Announce, -1, 0, line);
        }
    }

    pub fn priv_chat(&mut self, player: PlayerId, message: &str) {
        for line in chat_lines(message) {
            self.chat(ChatType::PrivateMessage, player.0 as i32, 0, line);
        }
    }

    pub fn chat(&mut self, kind: ChatType, speaker: i32, volume: i32, message: &str) {
        for line in chat_lines(message) {
            self.send_chat(line, kind, speaker, volume);
        }
    }

    pub fn player_ids(&self) -> Vec<PlayerId> {
        self.players.iter().map(|(_, p)| p.player_id).collect()
    }

    pub fn player(&self, player: PlayerId) -> Option<&Player> {
        self.players.get(player.idx())
    }

    pub fn mode_name(&self) -> &'static str {
        self.mode.name()
    }

    pub fn gravity_scale(&self) -> f32 {
        self.bodies.gravity_scale
    }

    pub fn set_gravity_scale(&mut self, scale: f32) {
        self.bodies.gravity_scale = scale;

        for (_, item) in self.items.iter_mut() {
            item.physics_settled = false;
            item.settled_timer = 0;
        }
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

    pub fn set_menu(&mut self, player: PlayerId, menu: u8) -> bool {
        let (Ok(menu), Some(p)) = (MenuType::try_from(menu as u32), self.players.get_mut(player.idx())) else { return false };
        p.menu = menu;
        true
    }

    pub fn human_ids(&self) -> Vec<usize> {
        self.humans.ids()
    }

    pub fn human(&self, id: usize) -> Option<&Human> {
        self.humans.get(id)
    }

    pub fn place_human(&mut self, pos: Vec3, rot: RotMatrix, player: Option<PlayerId>) -> Option<usize> {
        self.spawn_human(pos, &rot, player)
    }

    pub fn set_player_human(&mut self, player: PlayerId, human: usize) {
        let Some(p) = self.players.get_mut(player.idx()) else { return };
        p.human = Some(human);
        p.ghost_human = false;
        p.items_bought = 0;

        self.update_player_event(player);
    }

    pub fn human_mut(&mut self, id: usize) -> Option<&mut Human> {
        self.humans.get_mut(id)
    }

    pub fn item_kind_names(&self) -> Vec<String> {
        (0..ItemKind::COUNT as u8).filter_map(|k| ItemKind::try_from(k).ok()).map(|k| format!("{k:?}").to_lowercase()).collect()
    }

    pub fn find_item_kind(&self, name: &str) -> Option<u32> {
        self.item_kind_names().iter().position(|k| k.eq_ignore_ascii_case(name)).map(|k| k as u32)
    }

    pub fn spawn_item(&mut self, kind: u32, pos: Vec3, rot: RotMatrix) -> Option<usize> {
        let kind = ItemKind::try_from(u8::try_from(kind).ok()?).ok()?;
        self.create_item(kind, pos, None, rot)
    }

    pub fn item_ids(&self) -> Vec<usize> {
        self.items.ids()
    }

    pub fn item(&self, id: usize) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn remove_item(&mut self, id: usize) {
        self.delete_item(id);
    }

    pub fn phone_number(&self, item: usize) -> Option<i32> {
        self.items.get(item).and_then(|i| i.state.phone()).map(|p| p.number)
    }

    pub fn set_phone_number(&mut self, item: usize, number: i32) -> bool {
        let Some(phone) = self.items.get_mut(item).and_then(|i| i.state.phone_mut()) else { return false };
        phone.number = number;
        true
    }

    pub fn vehicle_kind_names(&self) -> Vec<(u32, String)> {
        self.vehicle_types.iter().enumerate().filter(|(_, t)| !t.nodes.is_empty()).map(|(k, t)| (k as u32, t.name.clone())).collect()
    }

    pub fn place_vehicle(&mut self, kind: u32, colour: i32, centre: Vec3, rot: RotMatrix) -> Option<usize> {
        let kind = VehicleKind::try_from(u8::try_from(kind).ok()?).ok()?;
        let com = self.vehicle_types.get(kind as usize).filter(|t| !t.nodes.is_empty())?.center_of_mass;

        self.spawn_vehicle(kind, colour, centre - rotate(&rot, com), rot)
    }

    pub fn ground_below(&self, pos: Vec3) -> Option<f32> {
        let map = &self.world.map;
        line_intersect_level(&map.ground, &map.level.area, &map.level.meshes, pos + Vec3::Y * 8.0, pos - Vec3::Y * 256.0).map(|g| g.hit.pos.y)
    }

    pub fn vehicle_ids(&self) -> Vec<usize> {
        self.vehicles.ids()
    }

    pub fn vehicle(&self, id: usize) -> Option<&Vehicle> {
        self.vehicles.get(id)
    }

    pub fn vehicle_pos(&self, id: usize) -> Option<Vec3> {
        self.bodies.get(self.vehicles.get(id)?.body).map(|b| b.pos)
    }

    pub fn remove_vehicle(&mut self, id: usize) {
        self.delete_vehicle(id);
    }

    pub fn remove_human(&mut self, id: usize) {
        self.delete_human(id);
    }

    pub fn clear_bullets(&mut self) {
        self.bullets.clear();
    }

    pub fn body_ids(&self) -> Vec<usize> {
        self.bodies.ids()
    }

    pub fn body(&self, id: usize) -> Option<&RigidBody> {
        self.bodies.get(id)
    }

    pub fn body_mut(&mut self, id: usize) -> Option<&mut RigidBody> {
        self.bodies.get_mut(id)
    }

    pub fn add_impulse(&mut self, id: usize, offset: Vec3, impulse: Vec3) {
        self.bodies.add_impulse(id, offset, impulse);
    }
}
