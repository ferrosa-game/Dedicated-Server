use ferrosa_wit::{GameState, Human, Menu, Player, SuitColour, Team, server};

use crate::{
    Round, TEAM_NAMES,
    bases::{Area, base},
    inventory::SLOTS,
    round_corp, spawn_corp_vehicle,
};


const COLOUR_NAMES: [&str; 6] = ["Black", "Red", "Blue", "Silver", "White", "Gold"];
/// A sold-out vehicle slot's button.
const SOLD: i32 = 8;
/// Button bytes 128 to 133 switch to menus 14 to 19.
const TAB: u32 = 0x80;
const TABS: [Menu; 6] = [Menu::RoundCorpWeapons, Menu::RoundCorpAmmo, Menu::RoundCorpEquip, Menu::RoundCorpVehicle, Menu::RoundCorpStock, Menu::WorldEmptyCorp];
/// The manager page (menu 19) and its fire button.
const MANAGER_PAGE: Menu = Menu::WorldEmptyCorp;
const FIRE: i32 = 6;
/// A player can buy 10 items a round day.
const BUY_LIMIT: i32 = 9;
const WEAPONS: [&str; 5] = ["pistol", "mp5", "ak47", "m16", "uzi"];
const AMMO: [&str; 5] = ["pistolmag", "mp5mag", "ak47mag", "m16mag", "uzimag"];
/// The suits on the equipment page: buttons 1 to 3 (colour, price).
const SUITS: [(SuitColour, i32); 3] = [(SuitColour::Navy, 5000), (SuitColour::Red, 20000), (SuitColour::White, 100000)];
/// The necklaces on the equipment page: buttons 4 and 5 (necklace, price).
const NECKLACES: [(u8, i32); 2] = [(1, 10000), (2, 1000000)];
/// A corporation records up to 128 fired accounts.
const MAX_FIRED: usize = 128;
/// The share page's buttons 0 to 3 buy this many shares, 4 to 7 sell them.
const SHARE_AMOUNTS: [i32; 4] = [1, 10, 100, 1000];
/// Hands for a briefcase, pockets for everything else.
const HANDS: std::ops::Range<u32> = 0..2;
const POCKETS: std::ops::Range<u32> = 2..7;
/// What a fired player loses from their human: guns, magazines, keys, grenades and bandages.
const STRIPPED: [&str; 3] = ["key", "grenade", "bandage"];

/// One vehicle on sale: whether it is still there, its kind, colour and price.
#[derive(Clone, Copy, Default)]
pub(crate) struct VehicleOffer {
    pub(crate) active: bool,
    pub(crate) kind: u32,
    pub(crate) colour: i32,
    pub(crate) price: i32,
}

pub(crate) fn is_buy_menu(menu: Menu) -> bool {
    TABS.contains(&menu)
}

#[derive(Clone, Copy)]
pub(crate) enum VehicleKind {
    TownCar,
    Turbo,
    TurboS,
    Beamer,
    Van,
    Minivan,
    Hatchback,
}

impl VehicleKind {
    fn name(self) -> &'static str {
        match self {
            Self::TownCar => "towncar",
            Self::Turbo => "turbo",
            Self::TurboS => "turbos",
            Self::Beamer => "beamer",
            Self::Van => "van",
            Self::Minivan => "minivan",
            Self::Hatchback => "hatchback",
        }
    }

    pub(crate) fn id(self) -> Option<u32> {
        server::find_vehicle_kind(self.name())
    }
}

fn random_stock_vehicle() -> VehicleKind {
    match rand::random_range(0..16) {
        0..=1 => VehicleKind::Van,
        2..=5 => VehicleKind::TownCar,
        6..=9 => VehicleKind::Hatchback,
        10..=12 => VehicleKind::Minivan,
        13..=14 => VehicleKind::Beamer,
        _ if rand::random_bool(1.0 / 16.0) => VehicleKind::TurboS,
        _ => VehicleKind::Turbo,
    }
}

pub(crate) fn randomize_stock(stock: &mut [VehicleOffer]) {
    for offer in stock.iter_mut().filter(|o| o.active) {
        if rand::random_bool(0.125) {
            offer.active = false;
        }
    }

    for offer in stock.iter_mut().filter(|o| !o.active) {
        let Some(kind) = random_stock_vehicle().id() else { continue };
        let colour = rand::random_range(0..6);

        *offer = VehicleOffer { active: true, kind, colour, price: server::vehicle_price(kind).unwrap_or(0) };
    }
}

impl Round {
    /// The menus logic_player leaves to the mode: in the lobby a player without a human has the lobby menu (and no
    /// menu otherwise), a corporation member inside their base gets the weapons page when no menu is open, and the buy
    /// pages close once they leave it.
    pub(crate) fn base_menus(&self) {
        let lobby = server::get_game_state() == GameState::Intermission;

        for p in server::players() {
            let Some(human) = p.human() else {
                p.set_menu(if lobby { Menu::Lobby } else { Menu::Empty });
                continue;
            };

            let at_base = round_corp(p.team()).and_then(base).is_some_and(|b| b.contains(human.pos()));

            match p.menu() {
                Menu::Empty if at_base => p.set_menu(Menu::RoundCorpWeapons),
                menu if !at_base && is_buy_menu(menu) => p.set_menu(Menu::Empty),
                _ => {}
            }
        }
    }

    pub(crate) fn menu_buttons(&self) {
        let players = server::players();
        for p in &players {
            match p.menu() {
                Menu::RoundCorpVehicle => {
                    for (i, o) in self.vehicle_stock.iter().enumerate() {
                        if !o.active {
                            p.add_menu_button(SOLD, "SOLD", Some(0));
                            continue;
                        }

                        let colour = usize::try_from(o.colour).ok().and_then(|c| COLOUR_NAMES.get(c)).copied().unwrap_or("");
                        let name = server::vehicle_display_name(o.kind).unwrap_or_default();

                        p.add_menu_button(i as i32, &format!("{colour} {name}"), Some(o.price));
                    }
                }
                MANAGER_PAGE if p.manager_tab() => {
                    for other in players.iter().filter(|o| o.id() != p.id() && o.team() == p.team()) {
                        p.add_menu_button(((other.id() as i32) << 16) + FIRE, &format!("Fire {}", other.name()), None);
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn buy_menu(&mut self, p: &Player, packed: u32) -> bool {
        let b = packed & 255;

        let Some(human) = p.human() else { return true };

        if (TAB..TAB + TABS.len() as u32).contains(&b) {
            p.set_menu(TABS[(b - TAB) as usize]);
        }

        let target = (packed as i32) >> 16;

        match (p.menu(), b) {
            (Menu::RoundCorpWeapons, 0..5) => self.buy_item(p, &human, WEAPONS[b as usize]),
            (Menu::RoundCorpAmmo, 0..5) => self.buy_item(p, &human, AMMO[b as usize]),
            (Menu::RoundCorpEquip, 0) => self.buy_item(p, &human, "bandage"),
            (Menu::RoundCorpEquip, 6) => self.buy_item(p, &human, "grenade"),
            (Menu::RoundCorpEquip, 7) => self.buy_item(p, &human, "briefcase"),
            (Menu::RoundCorpEquip, 1..=3) => {
                let (suit, price) = SUITS[b as usize - 1];

                if p.money() >= price {
                    p.set_money(p.money() - price);
                    p.set_suit_colour(suit);
                }
            }
            (Menu::RoundCorpEquip, 4 | 5) => {
                let (necklace, price) = NECKLACES[b as usize - 4];

                if p.money() >= price {
                    p.set_money(p.money() - price);
                    p.set_necklace(necklace);
                }
            }
            (Menu::RoundCorpVehicle, 0..8) => self.buy_vehicle(p, &human, b as usize),
            (Menu::RoundCorpStock, 0..4) => p.buy_stocks(SHARE_AMOUNTS[b as usize]),
            (Menu::RoundCorpStock, 4..8) => p.sell_stocks(SHARE_AMOUNTS[b as usize - 4]),
            (MANAGER_PAGE, 6) => self.fire(p, &human, target),
            _ => {}
        }

        true
    }

    fn buy_item(&mut self, p: &Player, human: &Human, name: &str) {
        let Some(kind) = server::find_item_kind(name) else { return };
        let price = server::item_price(kind).unwrap_or(0);
        let bought = self.items_bought.entry(p.id()).or_default();

        if *bought > BUY_LIMIT || !(p.money() >= price || price == 0) {
            return;
        }

        let slots = if name == "briefcase" { HANDS } else { POCKETS };

        if human.give_item(kind, slots.start, slots.end).is_none() {
            return;
        }

        p.set_money(p.money() - price);
        *bought += 1;
    }

    fn buy_vehicle(&mut self, p: &Player, human: &Human, slot: usize) {
        let offer = self.vehicle_stock[slot];
        let Some(team) = round_corp(p.team()) else { return };

        if !offer.active || p.money() < offer.price || !base(team).is_some_and(|b| b.contains(human.pos())) {
            return;
        }

        let Some(vehicle) = spawn_corp_vehicle(offer.kind, team, offer.colour) else { return };

        if let Some(key) = server::find_item_kind("key").and_then(|k| human.give_item(k, POCKETS.start, POCKETS.end)) {
            key.set_key_vehicle(Some(&vehicle));
        }

        p.set_money(p.money() - offer.price);
        self.vehicle_stock[slot].active = false;
    }

    fn fire(&mut self, p: &Player, human: &Human, target: i32) {
        let Some(team) = round_corp(p.team()) else { return };

        if self.managers[team] != Some(p.id()) {
            return;
        }

        let at_base = base(team).is_some_and(|b| b.contains(human.pos()));

        if !at_base || self.fired[team].len() >= MAX_FIRED || target == p.id() as i32 {
            return;
        }

        let Some(t) = u32::try_from(target).ok().and_then(server::get_player) else { return };

        if t.team() != p.team() {
            return;
        }

        if let Some(account) = t.account_id() {
            self.fired[team].push(account);
        }

        if let Some(h) = t.human() {
            strip_human(&h);
            h.delete();
        }

        t.sell_all_stocks();
        t.set_team(Team::Spectator);
        t.set_manager_tab(false);
        server::announce(&format!("{} has been fired from {}", t.name(), TEAM_NAMES[team]));
    }
}

fn strip_human(human: &Human) {
    let stripped: Vec<u32> = WEAPONS.iter().chain(&AMMO).chain(&STRIPPED).filter_map(|n| server::find_item_kind(n)).collect();

    for slot in 0..SLOTS {
        for item in human.slot(slot).into_iter().filter(|i| stripped.contains(&i.kind())) {
            for child in item.children() {
                child.delete();
            }

            item.delete();
        }
    }
}
