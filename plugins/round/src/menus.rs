use ferrosa_wit::{GameState, Human, Menu, Player, PlayerId, SuitColour, Team, server};

use crate::{
    Round, TEAM_NAMES,
    bases::{Area, base},
    round_corp, spawn_corp_vehicle,
};


const COLOUR_NAMES: [&str; 6] = ["Black", "Red", "Blue", "Silver", "White", "Gold"];
const TAB: u32 = 0x80;
const TABS: [Menu; 6] = [Menu::RoundCorpWeapons, Menu::RoundCorpAmmo, Menu::RoundCorpEquip, Menu::RoundCorpVehicle, Menu::RoundCorpStock, Menu::WorldEmptyCorp];
const WEAPONS: [(&str, &str); 5] = [("pistol", "9mm"), ("mp5", "MP5"), ("ak47", "AK-47"), ("m16", "M-16"), ("uzi", "Uzi")];
const AMMO: [(&str, &str); 5] = [
    ("pistolmag", "9mm Magazine"),
    ("mp5mag", "MP5 Magazine"),
    ("ak47mag", "AK-47 Magazine"),
    ("m16mag", "M-16 Magazine"),
    ("uzimag", "Uzi Magazine"),
];
const EQUIPMENT: [(&str, &str); 3] = [("bandage", "Bandage"), ("grenade", "Grenade"), ("briefcase", "Briefcase")];
const SUITS: [(SuitColour, i32, &str); 3] =
    [(SuitColour::Navy, 5000, "Navy Suit"), (SuitColour::Red, 20000, "Red Suit"), (SuitColour::White, 100000, "White Suit")];
const NECKLACES: [(u8, i32, &str); 2] = [(1, 10000, "Gold Chain"), (2, 1000000, "Diamond Chain")];
const SHARE_AMOUNTS: [i32; 4] = [1, 10, 100, 1000];

#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) enum Tab {
    #[default]
    Weapons,
    Ammo,
    Equipment,
    Vehicles,
    Stocks,
    Manager,
}

impl Tab {
    fn pressed(button: u32) -> Option<Self> {
        Some(match button.checked_sub(TAB)? {
            0 => Self::Weapons,
            1 => Self::Ammo,
            2 => Self::Equipment,
            3 => Self::Vehicles,
            4 => Self::Stocks,
            5 => Self::Manager,
            _ => return None,
        })
    }

    fn menu(self) -> Menu {
        match self {
            Self::Stocks | Self::Manager => Menu::WorldEmptyCorp,
            _ => Menu::RoundCorpVehicle,
        }
    }
}

fn shop_buttons(p: &Player, stock: &[(&str, &str)]) {
    for (i, (name, label)) in stock.iter().enumerate() {
        let price = server::find_item_kind(name).and_then(server::item_price).unwrap_or(0);

        p.add_menu_button(i as i32, label, Some(price));
    }
}

fn equipment_buttons(p: &Player) {
    shop_buttons(p, &EQUIPMENT);

    for (i, (_, price, label)) in SUITS.iter().enumerate() {
        p.add_menu_button((EQUIPMENT.len() + i) as i32, label, Some(*price));
    }

    for (i, (_, price, label)) in NECKLACES.iter().enumerate() {
        p.add_menu_button((EQUIPMENT.len() + SUITS.len() + i) as i32, label, Some(*price));
    }
}

fn stock_buttons(p: &Player) {
    for (i, n) in SHARE_AMOUNTS.into_iter().enumerate() {
        let s = if n == 1 { "" } else { "s" };

        p.add_menu_button(i as i32, &format!("Buy {n} Share{s}"), Some(0));
        p.add_menu_button((SHARE_AMOUNTS.len() + i) as i32, &format!("Sell {n} Share{s}"), Some(0));
    }
}

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
    pub(crate) fn corp_buy_menus(&self) {
        let lobby = server::get_game_state() == GameState::Intermission;

        for p in server::players() {
            let Some(human) = p.human() else {
                p.set_menu(if lobby { Menu::Lobby } else { Menu::Empty });
                continue;
            };

            let at_base = round_corp(p.team()).and_then(base).is_some_and(|b| b.contains(human.pos()));

            match p.menu() {
                Menu::Empty if at_base => p.set_menu(self.tab(p.id()).menu()),
                menu if !at_base && is_buy_menu(menu) => p.set_menu(Menu::Empty),
                _ => {}
            }
        }
    }

    pub(crate) fn tab(&self, player: PlayerId) -> Tab {
        self.shop_tab.get(&player).copied().unwrap_or_default()
    }

    pub(crate) fn menu_buttons(&self) {
        let players = server::players();

        for p in &players {
            if !is_buy_menu(p.menu()) {
                continue;
            }

            match self.tab(p.id()) {
                Tab::Weapons => shop_buttons(p, &WEAPONS),
                Tab::Ammo => shop_buttons(p, &AMMO),
                Tab::Equipment => equipment_buttons(p),
                Tab::Stocks => stock_buttons(p),
                Tab::Vehicles => {
                    for (i, o) in self.vehicle_stock.iter().enumerate() {
                        if !o.active {
                            p.add_menu_button(8, "SOLD", Some(0));
                            continue;
                        }

                        let colour = usize::try_from(o.colour).ok().and_then(|c| COLOUR_NAMES.get(c)).copied().unwrap_or("");
                        let name = server::vehicle_display_name(o.kind).unwrap_or_default();

                        p.add_menu_button(i as i32, &format!("{colour} {name}"), Some(o.price));
                    }
                }
                Tab::Manager if p.manager_tab() => {
                    for other in players.iter().filter(|o| o.id() != p.id() && o.team() == p.team()) {
                        p.add_menu_button(((other.id() as i32) << 16) + 6, &format!("Fire {}", other.name()), Some(0));
                    }
                }
                Tab::Manager => {}
            }
        }
    }

    pub(crate) fn buy_menu(&mut self, p: &Player, packed: u32) -> bool {
        let b = packed & 255;

        let Some(human) = p.human() else { return true };

        if let Some(tab) = Tab::pressed(b) {
            self.shop_tab.insert(p.id(), tab);
            p.set_menu(tab.menu());

            return true;
        }

        let target = (packed as i32) >> 16;
        let suit = EQUIPMENT.len() as u32;
        let necklace = suit + SUITS.len() as u32;
        let sell = SHARE_AMOUNTS.len() as u32;

        match (self.tab(p.id()), b) {
            (Tab::Weapons, 0..5) => self.buy_item(p, &human, WEAPONS[b as usize].0),
            (Tab::Ammo, 0..5) => self.buy_item(p, &human, AMMO[b as usize].0),
            (Tab::Equipment, _) if b < suit => self.buy_item(p, &human, EQUIPMENT[b as usize].0),
            (Tab::Equipment, _) if b < necklace => {
                let (colour, price, _) = SUITS[(b - suit) as usize];

                if p.money() >= price {
                    p.set_money(p.money() - price);
                    p.set_suit_colour(colour);
                }
            }
            (Tab::Equipment, _) if b < necklace + NECKLACES.len() as u32 => {
                let (chain, price, _) = NECKLACES[(b - necklace) as usize];

                if p.money() >= price {
                    p.set_money(p.money() - price);
                    p.set_necklace(chain);
                }
            }
            (Tab::Vehicles, 0..8) => self.buy_vehicle(p, &human, b as usize),
            (Tab::Stocks, _) if b < sell => p.buy_stocks(SHARE_AMOUNTS[b as usize]),
            (Tab::Stocks, _) if b < sell + sell => p.sell_stocks(SHARE_AMOUNTS[(b - sell) as usize]),
            (Tab::Manager, 6) => self.fire(p, &human, target),
            _ => {}
        }

        true
    }

    fn buy_item(&mut self, p: &Player, human: &Human, name: &str) {
        let Some(kind) = server::find_item_kind(name) else { return };
        let price = server::item_price(kind).unwrap_or(0);
        let bought = self.items_bought.entry(p.id()).or_default();

        if *bought > 9 || !(p.money() >= price || price == 0) {
            return;
        }

        let slots = if name == "briefcase" { 0..2 } else { 2..7 };

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

        if let Some(key) = server::find_item_kind("key").and_then(|k| human.give_item(k, 2, 7)) {
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

        if !at_base || self.fired[team].len() >= 128 || target == p.id() as i32 {
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
    let carried = WEAPONS.iter().chain(&AMMO).map(|(name, _)| *name).chain(["key", "grenade", "bandage"]);
    let stripped: Vec<u32> = carried.filter_map(server::find_item_kind).collect();

    for slot in 0..7 {
        for item in human.slot(slot).into_iter().filter(|i| stripped.contains(&i.kind())) {
            for child in item.children() {
                child.delete();
            }

            item.delete();
        }
    }
}
