use ferrosa_protocol::{
    Team,
    clientbound::game::{
        GameState, ItemKind,
        events::{Event, ServerEvent, team_door::EventTeamDoor},
    },
};
use glam::Vec3;

use super::Sim;
use crate::{
    PlayerId,
    items::logic::phone::PhoneStatus,
    world::{area::MESH, city_objects::GARAGE_DOOR, trace::line_intersect_level},
};

/// The team phones: the manager's at the table is numbered (team + 1) * 1111, and the one after it goes with it.
/// A member can work the garage door from up to 4 outside the base.
const DOOR_REACH: f32 = 4.0;

/// A corporation's people and account: its manager and applicants, and its money.
#[derive(Clone, Debug, Default)]
pub struct CorpState {
    pub manager: Option<PlayerId>,
    pub applicants: Vec<PlayerId>,
    pub money: i32,
    pub door_open: bool,
    pub funds: i32,
    pub missions: [super::missions::Mission; super::missions::MISSION_SLOTS],
    pub player_count: i32,
    pub car_space_taken: Vec<bool>,
    /// The accounts the round manager fired (up to 128).
    pub fired: Vec<u32>,
    /// The intel the corporation can ask for (1 to 8 can request, 0 requested, 11 faxed, others
    /// none) and how many screens it has printed (4 at most).
    pub intel: i32,
    pub prints: i32,
}

pub(crate) fn team_index(team: Team) -> Option<usize> {
    Team::CORPORATIONS.iter().position(|&t| t == team)
}

impl Sim {
    /// remove_player_form_corp: takes the player off every corporation's list of applicants.
    pub(crate) fn remove_player_from_corps(&mut self, pid: PlayerId) {
        for c in &mut self.corp_state {
            if let Some(i) = c.applicants.iter().position(|&a| a == pid) {
                c.applicants.swap_remove(i);
            }
        }
    }


    pub(crate) fn disconnect_phone(&mut self, k: usize) {
        let number = (k as i32 + 1) * 1111;

        for id in self.items.ids() {
            let Some(item) = self.items.get(id).filter(|i| i.item_type == ItemKind::Phone) else { continue };
            let Some(phone) = item.state.phone().filter(|p| p.number == number || p.number == number + 1) else { continue };

            if let Some(other) = phone.connected {
                if let Some(o) = self.items.get_mut(other).and_then(|i| i.state.phone_mut()) {
                    o.status = PhoneStatus::Idle;
                    o.connected = None;
                    o.display_number = 0;
                }

                self.phone_update(other);
            }

            let Some(item) = self.items.get_mut(id) else { continue };

            if let Some(phone) = item.state.phone_mut() {
                phone.status = PhoneStatus::Idle;
                phone.connected = None;
            }

            self.mark_item_for_deletion(id);
        }
    }

    /// update_corp_player_ratings: a tenth of the corporation's money (doubled when in debt) goes to its members'
    /// corporate ratings, half to the manager and half split among the rest (all of it when one member), and the
    /// account is emptied.
    pub(crate) fn update_corp_player_ratings(&mut self, k: usize) {
        let team = Team::CORPORATIONS[k];
        let members = self.players.iter().filter(|(_, p)| p.team == team).count() as i32;

        if members == 0 {
            return;
        }

        let tenth = self.corp_state[k].money / 10;
        let share = if tenth < 0 { tenth * 2 } else { tenth };
        let (manager_share, member_share) = if members == 1 { (share, share) } else { (share / 2, share / 2 / (members - 1)) };
        let manager = self.corp_state[k].manager;

        for (i, p) in self.players.iter_mut().filter(|(_, p)| p.team == team) {
            p.corp_rating += if manager.is_some_and(|m| m.idx() == i) { manager_share } else { member_share };
        }

        self.corp_state[k].money = 0;
    }


    /// Opens or closes a corporation's garage door (create_event_toggle_team_door); the door cells follow at the end of
    /// the tick (apply_team_doors).
    pub(crate) fn set_team_door(&mut self, k: usize, open: bool) {
        self.corp_state[k].door_open = open;
        let e = EventTeamDoor { team_id: k as i32, door_open: open };

        self.events.push(Event { tick_created: self.tick, kind: ServerEvent::TeamDoor(e) });
    }

    /// corporation_update_door_blocks for every base: an open door's cells are emptied, a closed door's hold the door.
    pub(crate) fn apply_team_doors(&mut self) {
        let level = &mut self.world.map.level;

        for (k, base) in level.bases.iter().enumerate() {
            let Some((cells, dir)) = base.door else { continue };
            let word = if self.corp_state[k].door_open { 0 } else { dir << 10 | GARAGE_DOOR };

            for c in cells {
                level.area.set_object(c.x, c.y, c.z, word);
            }
        }
    }

    /// The door part of human_update_hand_grab_and_inventory: in a game, a corporation member within 4 of their base
    /// who looks at its closed garage door opens it, and at the open door's frame closes it.
    pub(crate) fn team_door_probe(&mut self, player: Option<PlayerId>, start: Vec3, end: Vec3, pos: Vec3) {
        let map = &self.world.map;
        let Some(hit) = line_intersect_level(&map.ground, &map.level.area, &map.level.meshes, start, end) else { return };

        if hit.area == -1 {
            return;
        }

        let word = map.level.area.collision_cell_resolved(hit.block.x, hit.block.y, hit.block.z) & 0xfcffffff;

        if self.gamestate != GameState::InGame || !self.rules.door_knocks {
            return;
        }

        let Some(k) = self.players.get(player.unwrap_or(PlayerId(0)).idx()).and_then(|p| team_index(p.team)) else { return };
        let base = &map.level.bases[k];
        let (a, b) = (base.interior_min, base.interior_max);

        if pos.x < a.x - DOOR_REACH || b.x + DOOR_REACH <= pos.x || pos.z < a.z - DOOR_REACH || DOOR_REACH + b.z <= pos.z {
            return;
        }

        let garage = map.level.tables.garage_door();
        let frame = (garage[0] | MESH) <= word && word <= (garage[3] | MESH);
        let door = hit.object == GARAGE_DOOR as i32;

        match (self.corp_state[k].door_open, door || frame, frame) {
            (false, true, _) => self.set_team_door(k, true),
            (true, _, true) => self.set_team_door(k, false),
            _ => {}
        }
    }
}
