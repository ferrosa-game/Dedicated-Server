use ferrosa_physics::rotation::IDENTITY;
use ferrosa_protocol::clientbound::game::{ItemKind, MenuType, VehicleKind};

use super::{Sim, corporations::team_index};
use crate::{PlayerId, items, items::state::ItemState};

/// reset_game sets the clock to 11:00 outside world mode.
const CLOCK_START: i32 = 2376000;
/// The sun's roll: a quarter turn's worth of 256 steps, from a sixteenth of a turn up, and a half turn's tilt.
const SUN_STEP: f64 = 1.0 / 256.0;
const SUN_SPAN: f64 = 0.25;
const SUN_BASE: i32 = 64;

/// One item kept from a player's last round: its type and two values (a gun's
/// loaded magazine and rounds, an item's uses, a key's vehicle type and colour, cash bills and codes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SavedItem {
    pub kind: i32,
    pub a: i32,
    pub b: i32,
}

impl Sim {
    /// The corporation player counts of logic_simulation: active players on a corporation team with a human (or
    /// without, where the rules count them).
    pub(crate) fn count_corp_players(&mut self) {
        for c in &mut self.corp_state {
            c.player_count = 0;
        }

        let humanless = self.rules.count_humanless_members;

        for (_, p) in self.players.iter() {
            if let Some(k) = team_index(p.team)
                && (p.human.is_some() || humanless)
            {
                self.corp_state[k].player_count += 1;
            }
        }
    }

    /// server_main's check before each pass of its loop: when people come to an empty server the game is reset.
    pub(crate) fn check_occupied(&mut self) {
        let occupied = self.players.iter().any(|(_, p)| !p.is_bot);

        if occupied && !self.occupied {
            self.reset_game();
        }

        self.occupied = occupied;
    }

    /// reset_game's weather outside world mode, rolled before the reseed: a sun angle (the first roll is thrown away),
    /// the clock at 11:00 and an axial tilt.
    pub(crate) fn roll_weather(&mut self) {
        let angle = (((rand::random_range(0..256) as i32 + SUN_BASE) as f64 * std::f64::consts::PI) * SUN_SPAN * SUN_STEP) as f32;
        let tilt = ((rand::random_range(0..256) as f64 * std::f64::consts::PI) * SUN_STEP) as f32;

        self.world_time = CLOCK_START;
        self.world.set_sun(angle, tilt);
    }

    /// The lobby part of reset_game outside world mode: every player back in the lobby without a human, the world
    /// emptied and the round counted.
    pub(crate) fn reset_lobby_players_and_world(&mut self) {
        for (_, player) in self.players.iter_mut() {
            player.items_bought = 0;
            player.vehicles_bought = 0;
            player.human = None;
            player.ghost_human = false;
            player.is_ready = false;
            player.menu = MenuType::Lobby;
        }

        self.clear_world();
        self.round_number += 1;
    }

    /// The world part of reset_game: every body, human, vehicle, item, bullet and traffic car gone (and with the
    /// vehicles, the bases' car spaces free), the event list started over and each connection's object and event state
    /// with it.
    pub(crate) fn clear_world(&mut self) {
        let gravity = self.bodies.gravity_scale;

        self.bodies = ferrosa_physics::RigidBodies::default();
        self.bodies.gravity_scale = gravity;
        self.humans = ferrosa_physics::Table::new(crate::human::MAX_HUMANS);
        self.items = ferrosa_physics::Table::new(items::MAX_ITEMS);
        self.vehicles = ferrosa_physics::Table::new(crate::vehicle::MAX_VEHICLES);
        self.bullets.clear();
        self.npcs.clear();

        for c in &mut self.corp_state {
            c.car_space_taken.clear();
        }

        self.traffic = crate::traffic::Traffic::new(&self.world.map.streets, self.world.map.map_name == "round");
        self.events.clear();

        for clients in self.clients.values_mut() {
            clients.reset_for_round();
        }
    }

    /// save_inventory: what the player's human holds, kept for their next human.
    pub(crate) fn save_inventory(&mut self, pid: PlayerId) {
        let Some(player) = self.players.get_mut(pid.idx()) else { return };
        player.saved_inventory = Default::default();

        let Some(human) = player.human.and_then(|h| self.humans.get(h)) else { return };

        let mut saved: [Vec<SavedItem>; 7] = Default::default();
        let mut keys = Vec::new();

        for (slot, s) in human.inventory.iter().enumerate() {
            for &id in &s.items[..s.count.max(0) as usize] {
                let Some(item) = self.items.get(id as usize) else { continue };
                let kind = item.item_type as i32;

                if kind <= ItemKind::Bandage as i32 {
                    let left = item.state.left();
                    let mut entry = SavedItem { kind, a: 0, b: item.state.left() };

                    if let Some(mag) = item.children.first().and_then(|&c| self.items.get(c)) {
                        let rounds = mag.state.left();
                        entry.a = 1;
                        entry.b = rounds;

                        if left > 0 && rounds <= self.item_types[mag.item_type as usize].magazine_ammo {
                            entry.b = rounds + 1;
                        }
                    }

                    saved[slot].push(entry);
                } else if item.item_type == ItemKind::Key {
                    if let ItemState::Key { vehicle: Some(v) } = item.state
                        && let Some(veh) = self.vehicles.get(v).filter(|v| v.health > 0)
                    {
                        if self.rules.keys_keep_vehicles {
                            keys.push((slot, v));
                            saved[slot].push(SavedItem { kind, a: v as i32, b: 0 });
                        } else {
                            saved[slot].push(SavedItem { kind, a: veh.kind as i32, b: veh.color });
                        }
                    }
                } else if let ItemState::Cash(c) = &item.state {
                    saved[slot].push(SavedItem { kind, a: c.bills, b: c.codes as i32 });
                }
            }
        }

        for (_, v) in keys {
            self.save_world_key(v);
        }

        if let Some(p) = self.players.get_mut(pid.idx()) {
            p.saved_inventory = saved;
        }
    }

    /// init_player_inventory: the player's kept items back on their new human (a gun with its magazine in, a key with
    /// its vehicle respawned in the base).
    pub(crate) fn init_player_inventory(&mut self, pid: PlayerId) {
        let Some(p) = self.players.get(pid.idx()) else { return };
        let Some(human) = p.human else { return };
        let team = p.team;
        let saved = p.saved_inventory.clone();
        let Some(pos) = self.humans.get(human).map(|h| h.bones[0].pos) else { return };

        for (slot, items) in saved.iter().enumerate() {
            for saved_item in items {
                let Ok(kind) = ItemKind::try_from(saved_item.kind as u8) else { continue };

                let item = if saved_item.kind <= ItemKind::Bandage as i32 {
                    let Some(id) = self.create_item(kind, pos, None, IDENTITY) else { continue };

                    if !self.item_types[saved_item.kind as usize].is_gun {
                        if let Some(i) = self.items.get_mut(id) {
                            i.state.set_left(saved_item.b);
                        }
                    } else if saved_item.a > 0
                        && let Ok(mag_kind) = ItemKind::try_from(saved_item.kind as u8 + 1)
                        && let Some(mag) = self.create_item(mag_kind, pos, None, IDENTITY)
                    {
                        if let Some(i) = self.items.get_mut(mag) {
                            i.state.set_left(saved_item.b);
                        }

                        items::attach_child(&mut self.items, &self.item_types, id, mag);
                    }

                    id
                } else if kind == ItemKind::Key && self.rules.keys_keep_vehicles {
                    let Some(id) = self.create_item(ItemKind::Key, pos, None, IDENTITY) else { continue };
                    let v = self.world_key_vehicle(saved_item.a);

                    if let Some(i) = self.items.get_mut(id) {
                        i.state = ItemState::Key { vehicle: v };
                    }

                    id
                } else if kind == ItemKind::Key {
                    let Some(k) = team_index(team) else { continue };
                    let Some(kind) = u8::try_from(saved_item.a).ok().and_then(|a| VehicleKind::try_from(a).ok()) else { continue };
                    let Some(v) = self.corporation_spawn_vehicle(kind, k, saved_item.b) else { continue };
                    let Some(id) = self.create_item(ItemKind::Key, pos, None, IDENTITY) else { continue };

                    if let Some(i) = self.items.get_mut(id) {
                        i.state = ItemState::Key { vehicle: Some(v) };
                    }

                    id
                } else if kind == ItemKind::CashWorld {
                    let Some(id) = self.create_item(kind, pos, None, IDENTITY) else { continue };

                    if let Some(c) = self.items.get_mut(id).and_then(|i| i.state.cash_mut()) {
                        c.bills = saved_item.a;
                        c.codes = saved_item.b as u32;
                    }

                    id
                } else {
                    continue;
                };

                self.link_to_slot(human, item, slot);
            }
        }
    }

    /// corporation_spawn_vehicle: a vehicle in the first free car space of the corporation's base.
    pub(crate) fn corporation_spawn_vehicle(&mut self, kind: VehicleKind, k: usize, color: i32) -> Option<usize> {
        let spaces = self.world.map.level.bases.get(k)?.car_spaces.clone();
        let taken = &mut self.corp_state[k].car_space_taken;
        taken.resize(spaces.len(), false);

        let i = taken.iter().position(|t| !t)?;
        taken[i] = true;

        let (pos, rot) = spaces[i];
        self.spawn_vehicle(kind, color, pos, rot)
    }

    /// The base menus of every mode but world, for a human in no shop or bank: a corporation member inside their base
    /// gets the weapons page when no menu is open, where the rules give bases buy menus; anywhere else the menu closes.
    pub(crate) fn base_buy_menu(&mut self, pid: PlayerId, outside: Option<glam::Vec3>) {
        let Some(pos) = outside else { return };
        let at_base = self.rules.corp_buy_menus
            && self.players.get(pid.idx()).and_then(|p| team_index(p.team)).is_some_and(|k| self.world.map.level.bases.get(k).is_some_and(|b| b.contains(pos)));
        let Some(p) = self.players.get_mut(pid.idx()) else { return };

        if !at_base {
            p.menu = MenuType::Empty;
        } else if p.menu == MenuType::Empty {
            p.menu = MenuType::RoundCorpWeapons;
        }
    }

}
