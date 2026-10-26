use ferrosa_physics::rotation::{IDENTITY, rotate_orientation};
use ferrosa_protocol::{SuitColor, clientbound::game::MenuType};
use glam::Vec3;

use ferrosa_protocol::clientbound::game::ItemKind;

use super::{Sim, items::touchables};
use crate::items::state::ItemState;
use crate::{
    PlayerId,
    world::building::{BURGER_SHOP, CAR_DEALER, CLOTHING_STORE, GUN_STORE, ShopEntry},
};

fn menu(id: u8) -> MenuType {
    match id {
        9 => MenuType::WorldCarShop,
        10 => MenuType::WorldStore,
        11 => MenuType::WorldStoreDone,
        13 => MenuType::WorldBank2,
        _ => MenuType::Empty,
    }
}

impl Sim {



    /// The menu part of logic_player: standing in a dealership opens its front page (menu 9), in a store, clothing
    /// store or burger shop its list (menu 10), in a bank the bank (menu 13); leaving the building closes menus 9 to 11.
    /// The mode then builds each player's base menus.
    pub(crate) fn update_shop_menus(&mut self) {
        let mode = self.mode;

        for pid in self.players.iter().map(|(_, p)| p.player_id).collect::<Vec<_>>() {
            let outside = self.shop_menu(pid).flatten();
            mode.base_menus(self, pid, outside);
        }

        mode.menus_done(self);

        let plugins = self.plugins;
        plugins.menus_done(self);
    }

    /// One player's shop menu. Returns None for a player without a human, and the human's position when it is in no
    /// shop or bank (where world mode then closes every menu outside a corporation base), except with the /stocks test
    /// menu open, which is kept.
    fn shop_menu(&mut self, pid: PlayerId) -> Option<Option<glam::Vec3>> {
        let buildings = &self.world.map.level.buildings;
        let restarting = self.gamestate == ferrosa_protocol::clientbound::game::GameState::Restarting;
        let p = self.players.get_mut(pid.idx())?;
        p.menu_buttons.clear();

        if p.ghost_human && p.human.is_none() {
            p.menu = MenuType::Empty;
            return None;
        }

        if p.human.is_none() {
            if self.rules.intermission_lobby {
                p.menu = if self.gamestate == ferrosa_protocol::clientbound::game::GameState::Intermission { MenuType::Lobby } else { MenuType::Empty };
            }

            return None;
        }

        let pos = p.human.and_then(|h| self.humans.get(h)).map(|h| h.pos)?;

        if restarting {
            p.menu = MenuType::Empty;
            return Some(None);
        }

        let id = p.menu as u8;

        if (9..=11).contains(&id) && !usize::try_from(p.menu_tab).ok().and_then(|k| buildings.get(k)).is_some_and(|b| b.contains(pos, 0.0)) {
            p.menu = MenuType::Empty;
        }

        let shop = buildings.iter().enumerate().find_map(|(k, b)| match b.kind {
            CAR_DEALER if b.contains(pos, 0.0) => Some((k, 9)),
            GUN_STORE | BURGER_SHOP | CLOTHING_STORE if b.contains(pos, 0.0) => Some((k, 10)),
            _ => None,
        });
        let found = shop.or_else(|| buildings.iter().position(|b| b.kind == crate::world::building::BANK && b.contains(pos, 0.0)).map(|k| (k, 13)));
        let Some((k, m)) = found else {
            return Some((p.menu != MenuType::RoundCorpStock).then_some(pos));
        };

        if p.menu == MenuType::Empty {
            p.menu = menu(m);
            p.menu_tab = k as i32;
        }

        Some(None)
    }

    /// The shop menus of logic_playerinteractions: the front page picks buying (1) or selling (2); a list buys its entry
    /// `button`, and the back button returns to the front page.
    pub(crate) fn shop_menu_action(&mut self, pid: PlayerId, button: u32) {
        let Some(p) = self.players.get(pid.idx()) else { return };

        if p.human.is_none() {
            return;
        }

        match p.menu as u8 {
            9 => {
                let p = self.players.get_mut(pid.idx()).unwrap();

                match button {
                    1 => p.menu = MenuType::WorldStore,
                    2 => p.menu = MenuType::WorldStoreDone,
                    _ => {}
                }
            }
            10 => {
                let Some(b) = usize::try_from(p.menu_tab).ok().and_then(|k| self.world.map.level.buildings.get(k)) else { return };
                let (kind, count) = (b.kind, b.shop.len() as u32);

                if button > 0 && button <= count {
                    match kind {
                        5 => self.clothing_store_purchase(pid, button),
                        3 => self.dealership_buy_vehicle(pid, button),
                        8 if p.items_bought <= 19 => self.buy_shop_item(pid, button),
                        9 => self.burger_shop_menu(pid, button),
                        _ => {}
                    }
                }

                if button == 16
                    && let Some(p) = self.players.get_mut(pid.idx())
                {
                    p.menu = MenuType::WorldCarShop;
                }
            }
            11 => {
                if button == 1 {
                    self.player_sell_vehicle(pid);
                }

                if button == 16
                    && let Some(p) = self.players.get_mut(pid.idx())
                {
                    p.menu = MenuType::WorldCarShop;
                }
            }
            _ => {}
        }
    }

    /// with its key in the first free pocket, and the line goes (the last one takes its place).
    fn dealership_buy_vehicle(&mut self, pid: PlayerId, button: u32) {
        let Some(p) = self.players.get(pid.idx()) else { return };
        let Some(human) = p.human else { return };

        if p.vehicles_bought > 2 {
            return;
        }

        let Some(k) = usize::try_from(p.menu_tab).ok() else { return };
        let Some(b) = self.world.map.level.buildings.get_mut(k) else { return };
        let Some(e) = b.shop.get(button as usize - 1).copied() else { return };

        if p.money < e.price {
            return;
        }

        let n = b.car_counter;
        let d = (-n) as f32 * 4.0;
        let r2 = b.rot[2];
        let pos = r2 * d + b.car_spawn;
        b.car_counter = (n + 1) & 7;
        let mut rot = b.rot;
        rotate_orientation(&mut rot, Vec3::Y, 90.0_f32.to_radians());
        let kind = u8::try_from(e.kind).ok().and_then(|k| ferrosa_protocol::clientbound::game::VehicleKind::try_from(k).ok());
        let vid = kind.and_then(|kind| self.spawn_vehicle(kind, e.extra, pos, rot));

        if let Some(v) = vid.and_then(|v| self.vehicles.get_mut(v)) {
            v.owner = pid.i32();
        }

        if let Some(p) = self.players.get_mut(pid.idx()) {
            p.money -= e.price;
        }

        if let Some(key) = self.create_item(ItemKind::Key, pos, None, rot) {
            if let Some(i) = self.items.get_mut(key) {
                i.state = ItemState::Key { vehicle: vid };
            }

            self.link_first_free(human, key, 2..7);
        }

        if let Some(p) = self.players.get_mut(pid.idx()) {
            p.vehicles_bought += 1;

            self.update_player_event(pid);
        }

        if let Some(p) = self.players.get(pid.idx()) {
            let e = p.make_update_financials_event(self.tick);
            self.events.push(e);
        }

        if let Some(b) = self.world.map.level.buildings.get_mut(k) {
            b.shop.swap_remove(button as usize - 1);
        }
    }

    /// player_sell_vehicle: the key in the right hand sells its standing vehicle for three quarters of its price; the
    /// car and the key go.
    fn player_sell_vehicle(&mut self, pid: PlayerId) {
        let Some(h) = self.players.get(pid.idx()).and_then(|p| p.human).and_then(|h| self.humans.get(h)) else { return };
        let hand = &h.inventory[0];

        if hand.count <= 0 {
            return;
        }

        let key = hand.items[0] as usize;
        let Some(ItemState::Key { vehicle: Some(vid) }) = self.items.get(key).map(|i| i.state.clone()) else { return };
        let Some(v) = self.vehicles.get(vid).filter(|v| v.health > 0) else { return };
        let price = self.vehicle_types.get(v.kind as usize).map_or(0, |t| t.price);

        if let Some(p) = self.players.get_mut(pid.idx()) {
            p.money = (price as f32 * 0.75 + p.money as f32) as i32;
        }

        if let Some(v) = self.vehicles.get_mut(vid) {
            v.despawn_time = 0;
        }

        if let Some(i) = self.items.get_mut(key) {
            i.despawn_time = 0;
        }
    }

    fn shop_entry(&self, pid: PlayerId, button: u32) -> Option<ShopEntry> {
        let p = self.players.get(pid.idx())?;
        let b = self.world.map.level.buildings.get(usize::try_from(p.menu_tab).ok()?)?;
        b.shop.get((button as usize).checked_sub(1)?).copied()
    }

    /// The item put into the first of `slots` of the human that takes it.
    fn link_first_free(&mut self, human: usize, item: usize, slots: std::ops::Range<usize>) {
        let Some(h) = self.humans.get_mut(human) else { return };
        let mut touch = touchables!(self, Vec::new());

        for slot in slots {
            if crate::human::inventory::link_item_to_human(h, human, &mut self.bodies, &mut touch, item, slot) {
                break;
            }
        }
    }

    /// An item at the human, set up by `init` and put into the first of `slots` that takes it. Returns the item.
    pub(crate) fn give_item(&mut self, human: usize, kind: u8, slots: std::ops::Range<usize>, init: impl FnOnce(&mut ItemState)) -> Option<usize> {
        let kind = ItemKind::try_from(kind).ok()?;
        let pos = self.humans.get(human).map(|h| h.pos)?;
        let item = self.create_item(kind, pos, None, IDENTITY)?;

        if let Some(i) = self.items.get_mut(item) {
            init(&mut i.state);
        }

        self.link_first_free(human, item, slots);
        Some(item)
    }

    /// buy_shop_item: the entry's item into the first free pocket, paid for.
    fn buy_shop_item(&mut self, pid: PlayerId, button: u32) {
        let Some(entry) = self.shop_entry(pid, button) else { return };
        let Some(p) = self.players.get(pid.idx()) else { return };

        if entry.price > p.money {
            return;
        }

        if let Some(h) = p.human {
            self.give_item(h, entry.kind as u8, 2..7, |_| {});
        }

        let p = self.players.get_mut(pid.idx()).unwrap();
        p.money -= entry.price;
        p.items_bought += 1;

        self.update_player_event(pid);
    }

    /// burger_shop_menu: a burger (button 1) into the first slot that takes it, paid for.
    fn burger_shop_menu(&mut self, pid: PlayerId, button: u32) {
        let Some(entry) = self.shop_entry(pid, button) else { return };
        let Some(p) = self.players.get_mut(pid.idx()) else { return };

        if p.items_bought > 9 || p.money < entry.price {
            return;
        }

        p.money -= entry.price;
        p.items_bought += 1;
        if let Some(h) = p.human
            && button == 1
        {
            self.give_item(h, ItemKind::Burger as u8, 0..7, |_| {});
        }
    }

    /// clothing_store_purchase: buttons 1 to 3 change the suit's colour, 4 and 5 the necklace, each paid for only when
    /// it changes something.
    fn clothing_store_purchase(&mut self, pid: PlayerId, button: u32) {
        let Some(entry) = self.shop_entry(pid, button) else { return };
        let Some(player) = self.players.get_mut(pid.idx()) else { return };
        let Some(human) = player.human else { return };

        if player.money < entry.price {
            return;
        }

        let mut look = player.customization;

        match button {
            1..=3 => {
                let suit = match button {
                    1 => SuitColor::Navy,
                    2 => SuitColor::Red,
                    3 => SuitColor::White,
                    _ => SuitColor::Navy
                };

                if look.suit_color != suit {
                    look.suit_color = suit;
                    player.money -= entry.price;
                }
            }
            4 | 5 => {
                let necklace = (button != 4) as u8 + 1;

                if look.necklace != necklace {
                    look.necklace = necklace;
                    player.money -= entry.price;
                }
            }
            _ => {}
        }

        player.customization = look;

        if let Some(h) = self.humans.get_mut(human) {
            h.customization.suit_color = look.suit_color;
            h.customization.necklace = look.necklace;
        }

        self.update_player_event(pid);
    }
}
