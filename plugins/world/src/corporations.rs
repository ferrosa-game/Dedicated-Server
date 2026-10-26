use std::f32::consts::FRAC_PI_2;

use ferrosa_wit::{Base, Bill, Cash, ItemKind, Mat3, Menu, Model, PlayerId, SuitColour, Team, Vec3, server};

use crate::{World, state::CORPORATIONS, state::World as State};

const TAB: i32 = 0x1000000;
const TAB_HIRING: i32 = TAB | 1;
const TAB_FIRING: i32 = TAB | 2;
const TAB_TEAM: i32 = TAB | 3;
const TAB_REQUISITION: i32 = TAB | 4;

const REQUISITIONS: [(i32, &str, ItemKind); 8] = [
    (0, "Buy Disk", ItemKind::DiskBlack),
    (1, "Buy Briefcase", ItemKind::Briefcase),
    (6, "Buy Walkie-Talkie", ItemKind::Radio),
    (7, "Buy 9mm", ItemKind::Pistol),
    (8, "Buy 9mm Magazine", ItemKind::PistolMag),
    (9, "Buy MP5", ItemKind::Mp5),
    (10, "Buy MP5 Magazine", ItemKind::Mp5Mag),
    (11, "Buy Bandage", ItemKind::Bandage),
];

fn inside(base: &Base, pos: Vec3) -> bool {
    let (min, max) = (base.interior_min, base.interior_max);

    pos.x >= min.x && pos.x < max.x && pos.y >= min.y && pos.y < max.y && pos.z >= min.z && pos.z < max.z
}

fn base_at(pos: Vec3) -> Option<usize> {
    CORPORATIONS.iter().position(|&t| server::get_corporation(t).and_then(|c| c.base()).is_some_and(|b| inside(&b, pos)))
}

fn menu_of(tab: u8) -> Menu {
    match tab {
        20 => Menu::WorldCorpApplication,
        21 => Menu::WorldCorpTabs,
        22 => Menu::WorldCorpHiring,
        23 => Menu::WorldCorpFiring,
        24 => Menu::WorldCorpTeam,
        25 => Menu::WorldCorpRequisition,
        _ => Menu::Empty,
    }
}

fn menu_number(menu: Menu) -> u8 {
    match menu {
        Menu::WorldCorpApplication => 20,
        Menu::WorldCorpTabs => 21,
        Menu::WorldCorpHiring => 22,
        Menu::WorldCorpFiring => 23,
        Menu::WorldCorpTeam => 24,
        Menu::WorldCorpRequisition => 25,
        _ => 0,
    }
}

impl World {
    pub fn corp_base_menu(&mut self, pid: PlayerId, pos: Vec3) {
        let Some(p) = server::get_player(pid) else { return };

        let Some(k) = base_at(pos) else {
            p.set_menu(Menu::Empty);

            return;
        };

        let manager = self.state.corps[k].manager;
        let applied = self.state.corps[k].applicants.contains(&pid);

        if p.team() == Team::Spectator {
            p.set_menu(Menu::WorldCorpApplication);

            match manager {
                None => p.add_menu_button(0, "Apply Manager", None),
                Some(_) if !applied => {
                    p.add_menu_button(1, "Apply", None);
                    self.forget_applicant(pid);
                }
                Some(_) => {}
            }

            return;
        }

        let ours = State::team_index(p.team()) == Some(k);

        match manager {
            Some(_) if ours && matches!(p.menu(), Menu::WorldCorpApplication | Menu::Empty) => p.set_menu(Menu::WorldCorpRequisition),
            Some(_) => {}
            None => {
                if ours {
                    p.set_menu(Menu::WorldCorpApplication);
                }

                p.add_menu_button(0, "Apply Manager", None);
            }
        }
    }

    pub fn corp_tab_menu(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };
        let Some(k) = State::team_index(p.team()) else { return };
        let menu = menu_number(p.menu());

        if !(21..=26).contains(&menu) {
            return;
        }

        let tab = |id: i32, open: u8| if menu == open { -1 } else { id };
        let managing = self.state.corps[k].manager == Some(pid);

        p.add_menu_button(tab(TAB_REQUISITION, 25), "Requisition", None);

        if managing {
            p.add_menu_button(tab(TAB_HIRING, 22), "Hiring", None);
            p.add_menu_button(tab(TAB_FIRING, 23), "Firing", None);
            p.add_menu_button(tab(TAB_TEAM, 24), "Team", None);

            if menu == 22 {
                for &a in &self.state.corps[k].applicants {
                    let name = server::get_player(a).map(|o| o.name()).unwrap_or_default();

                    p.add_menu_button(((a as i32) << 16) + 3, &format!("Hire {name}"), None);
                }
            }

            if menu == 23 {
                for other in server::players().iter().filter(|o| o.id() != pid && o.team() == p.team()) {
                    p.add_menu_button(((other.id() as i32) << 16) + 6, &format!("Fire {}", other.name()), None);
                }
            }
        } else {
            p.add_menu_button(tab(TAB_TEAM, 24), "Team", None);
        }

        if menu == 25 {
            for (id, text, _) in REQUISITIONS {
                p.add_menu_button(id, text, None);
            }

            p.add_menu_button(2, "Deposit", None);

            if managing {
                p.add_menu_button(3, "Withdrawl $20", None);
                p.add_menu_button(4, "Withdrawl $100", None);
                p.add_menu_button(5, "Withdrawl $1000", None);
                p.add_menu_button(12, "Replace Phone", None);
            }
        }

        if menu == 24 {
            p.add_menu_button(0, "Quit Team", None);
        }
    }

    pub fn copy_corp_money(&mut self) {
        for p in server::players() {
            let corp = State::team_index(p.team()).map(|k| &self.state.corps[k]);

            p.set_corp_money(corp.map_or(0, |c| c.money), corp.map_or(0, |c| c.credit));
        }
    }

    pub fn forget_applicant(&mut self, pid: PlayerId) {
        for c in &mut self.state.corps {
            c.applicants.retain(|&a| a != pid);
        }
    }

    pub fn corp_menu_action(&mut self, pid: PlayerId, action: u8, b: i32) -> bool {
        let Some(p) = server::get_player(pid) else { return false };
        let Some(pos) = p.human().map(|h| h.pos()) else { return false };
        let Some(k) = base_at(pos) else { return false };

        if (21..=26).contains(&action) {
            if b & TAB != 0 {
                let menu = match b & 255 {
                    0 => 21,
                    1 => 22,
                    2 => 23,
                    3 => 24,
                    4 => 25,
                    _ => return true,
                };

                p.set_menu(menu_of(menu));

                return true;
            }

            if action == 24 && b == 0 && State::team_index(p.team()) == Some(k) {
                self.set_world_team(pid, Team::Spectator);

                return true;
            }
        }

        if State::team_index(p.team()) != Some(k) {
            if action == 20 {
                self.corp_apply(pid, k, b);
            }

            return true;
        }

        let manager = self.state.corps[k].manager;

        if manager == Some(pid) {
            match action {
                22 if b & 255 == 3 => self.corp_hire(k, (b >> 16) as PlayerId),
                22 => {}
                23 => self.corp_fire(pid, k, (b >> 16) as PlayerId),
                25 => self.corp_requisition(pid, k, b),
                20 => self.corp_apply(pid, k, b),
                _ => {}
            }

            return true;
        }

        match action {
            25 if manager.is_some() => self.corp_requisition(pid, k, b),
            20 => self.corp_apply(pid, k, b),
            _ => {}
        }

        true
    }

    fn corp_apply(&mut self, pid: PlayerId, k: usize, b: i32) {
        match b {
            0 if self.state.corps[k].manager.is_none() => {
                self.forget_applicant(pid);

                self.set_world_team(pid, CORPORATIONS[k]);

                if self.state.corps[k].manager.is_none() {
                    self.set_manager(k, pid);
                }

                if let Some(p) = server::get_player(pid) {
                    p.set_menu(Menu::WorldCorpRequisition);
                }

                self.broadcast_corp_missions(k);
                self.state.corps[k].applicants.clear();
            }

            1 if self.state.corps[k].manager.is_some() => {
                let list = &mut self.state.corps[k].applicants;

                if !list.contains(&pid) && list.len() < 32 {
                    list.push(pid);
                }
            }
            _ => {}
        }
    }

    pub fn set_manager(&mut self, k: usize, pid: PlayerId) {
        self.state.corps[k].manager = Some(pid);
        let rating = server::get_player(pid).map_or(0, |p| p.corp_rating());
        self.state.corps[k].credit = rating.max(0);
        self.scale_missions_for_manager(k, rating);

        let Some(corp) = server::get_corporation(CORPORATIONS[k]) else { return };

        if let Some(name) = server::get_player(pid).map(|p| p.name()) {
            server::announce(&format!("{name} is now manager of {}", corp.name()));
        }

        let Some(base) = corp.base() else { return };

        corp.disconnect_phones();

        let rot = Mat3::from_yaw(base.table_orientation - FRAC_PI_2);
        let Some(phone) = server::spawn_item_by_id(ItemKind::Phone as u32, base.table, rot) else { return };

        phone.set_phone_number((k as i32 + 1) * 1111);
        phone.set_phone_texture(0);
    }

    fn corp_hire(&mut self, k: usize, target: PlayerId) {
        if server::get_player(target).is_none() {
            return;
        }

        self.set_world_team(target, CORPORATIONS[k]);
        self.forget_applicant(target);
        self.broadcast_corp_missions(k);
    }

    fn corp_fire(&mut self, pid: PlayerId, k: usize, target: PlayerId) {
        if target == pid {
            return;
        }

        if !server::get_player(target).is_some_and(|p| State::team_index(p.team()) == Some(k)) {
            return;
        }

        self.set_world_team(target, Team::Spectator);
    }

    fn corp_requisition(&mut self, pid: PlayerId, k: usize, b: i32) {
        let Some(p) = server::get_player(pid) else { return };
        let Some(human) = p.human() else { return };

        let kind = match b {
            12 => Some(ItemKind::Phone),
            _ => REQUISITIONS.iter().find(|r| r.0 == b).map(|r| r.2),
        };

        if let Some(kind) = kind {
            if p.items_bought() > 9 {
                return;
            }

            if kind == ItemKind::Phone
                && let Some(corp) = server::get_corporation(CORPORATIONS[k])
            {
                corp.disconnect_phones();
            }

            let Some(item) = human.give_item(kind as u32, 0, 7) else { return };

            match kind {
                ItemKind::Radio => {
                    item.set_radio_channel(k as i32);
                }
                ItemKind::Phone => {
                    item.set_phone_number((k as i32 + 1) * 1111);
                    item.set_phone_texture(0);
                }
                ItemKind::DiskBlack => {
                    if let Some(volume) = server::new_volume(360) {
                        item.set_volume(&volume);
                    }
                }
                _ => {}
            }

            let price = if kind == ItemKind::Phone { 100 } else { server::item_price(kind as u32).unwrap_or(0) };
            self.state.corps[k].money -= price;
            p.set_items_bought(p.items_bought() + 1);

            return;
        }

        match b {
            2 => self.corp_deposit(pid, k),
            3..=5 if self.state.corps[k].manager == Some(pid) => self.corp_withdraw(pid, k, b),
            _ => {}
        }
    }

    fn corp_deposit(&mut self, pid: PlayerId, k: usize) {
        let Some(human) = server::get_player(pid).and_then(|p| p.human()) else { return };
        let Some(item) = human.slot(0).into_iter().next() else { return };
        let Some(value) = item.cash_value() else { return };

        self.state.corps[k].money += value;
        item.delete();
    }

    fn corp_withdraw(&mut self, pid: PlayerId, k: usize, b: i32) {
        let note = match b {
            4 => Bill::Hundred,
            5 => Bill::Thousand,
            _ => Bill::Twenty,
        };

        let value = server::bill_value(note);

        if self.state.corps[k].money - value < -self.state.corps[k].credit {
            return;
        }

        let Some(human) = server::get_player(pid).and_then(|p| p.human()) else { return };
        let right = human.slot(0).into_iter().next();
        let holding_other = right.as_ref().is_some_and(|i| i.kind() != ItemKind::CashWorld as u32);
        let slot = if holding_other { 1 } else { 0 };

        match human.slot(slot).into_iter().next() {
            None => {
                let Some(item) = human.give_item(ItemKind::CashWorld as u32, slot, slot + 1) else { return };

                item.set_cash(&Cash { bills: vec![note] });
            }
            Some(item) => {
                if !item.insert_bill(0, note) {
                    return;
                }
            }
        }

        self.state.corps[k].money -= value;
    }

    pub fn broadcast_corp_missions(&mut self, k: usize) {
        let Some(corp) = server::get_corporation(CORPORATIONS[k]) else { return };

        for (slot, m) in corp.missions().iter().enumerate() {
            if m.active {
                corp.send_mission_text(slot as u32);
            }
        }
    }
}

const SUIT_COLOURS: [SuitColour; 6] =
    [SuitColour::Tan, SuitColour::DarkNavy, SuitColour::DarkGrey, SuitColour::Grey, SuitColour::Black, SuitColour::Grey];
const TIE_COLOURS: [SuitColour; 6] =
    [SuitColour::Black, SuitColour::Red, SuitColour::White, SuitColour::Grey, SuitColour::Tan, SuitColour::Brown];

fn keeps_suit(suit: SuitColour) -> bool {
    matches!(suit, SuitColour::Navy | SuitColour::White | SuitColour::Red)
}

impl World {
    pub fn set_world_team(&mut self, pid: PlayerId, team: Team) {
        let Some(p) = server::get_player(pid) else { return };
        let old = p.team();

        if old == team {
            return;
        }

        self.leave_corp_management(pid, old);

        let Some(p) = server::get_player(pid) else { return };
        p.set_team(team);

        match State::team_index(team) {
            Some(k) => {
                p.set_model(Model::Suit);

                if !keeps_suit(p.suit_colour()) {
                    p.set_suit_colour(SUIT_COLOURS[k]);
                }

                p.set_tie_colour(Some(TIE_COLOURS[k]));
            }
            None if team == Team::Spectator => {
                p.set_model(Model::Casual);

                if !keeps_suit(p.suit_colour()) {
                    p.set_suit_colour(SuitColour::Brown);
                }

                p.set_tie_colour(None);
            }
            None => {}
        }
    }

    pub fn leave_corp_management(&mut self, pid: PlayerId, team: Team) {
        let Some(k) = State::team_index(team) else { return };

        if self.state.corps[k].manager != Some(pid) {
            return;
        }

        self.update_corp_player_ratings(k);
        self.state.corps[k].manager = None;
    }
}
