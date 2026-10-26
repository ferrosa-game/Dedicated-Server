use std::net::IpAddr;

use ferrosa_physics::rotation::IDENTITY;
use ferrosa_protocol::{
    Team,
    clientbound::{
        admin_list::AdminList,
        game::{ItemKind, VehicleKind, events::chat::ChatType},
    },
    frame_packet,
};

use super::{Sim, bans};
use crate::time::game_seconds;
use crate::{ConnId, PlayerId, items::logic::phone::PhoneStatus};

const ARM_GUNS: [ItemKind; 5] = [ItemKind::Ak47, ItemKind::M16, ItemKind::Mp5, ItemKind::Uzi, ItemKind::Pistol];

#[derive(Debug, Default)]
pub struct AdminState {
    pub password: String,
    pub phones: Vec<u32>,
    pub kicked: Vec<IpAddr>,
    /// The account an admin asked to ban, waiting for the /bany that says how long.
    pub pending_ban: Option<u32>,
    pub reset_requested: bool,
    pub next_mode: Option<crate::gamemode::Mode>,
}

impl AdminState {
    /// load_admin_data: every `adminphone=` number in serveradmin.txt. The `steamban=` ids beside it belong to the
    /// ban list.
    pub(crate) fn load(path: &std::path::Path) -> Self {
        let phones = std::fs::read_to_string(path)
            .map(|text| text.split("adminphone=").skip(1).filter_map(|rest| leading_int(rest).map(|n| n as u32)).collect())
            .unwrap_or_default();

        AdminState { phones, ..Default::default() }
    }
}

/// sscanf("%d"): Err for nothing but blanks (EOF), Ok(None) when no number starts the text.
fn scan_int(s: &str) -> Result<Option<i32>, ()> {
    let t = s.trim_start();

    if t.is_empty() {
        return Err(());
    }

    Ok(leading_int(t))
}

fn leading_int(s: &str) -> Option<i32> {
    let t = s.trim_start();
    let end = t.char_indices().find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')))).map_or(t.len(), |(i, _)| i);
    t[..end].parse::<i64>().ok().map(|v| v as i32)
}

/// The numbers in the text in turn, skipping whatever lies between them (get_actual_number and
/// opened_file_parse_next_int).
fn numbers(s: &str) -> impl Iterator<Item = i32> + '_ {
    let mut rest = s;

    std::iter::from_fn(move || {
        let start = rest.find(|c: char| c.is_ascii_digit() || c == '-')?;
        rest = &rest[start..];
        let n = leading_int(rest);
        let len = rest.char_indices().skip(1).find(|&(_, c)| !c.is_ascii_digit()).map_or(rest.len(), |(i, _)| i);
        rest = &rest[len..];
        n
    })
}

impl Sim {
    pub fn set_admin_password(&mut self, password: String) {
        self.admin.password = password;
    }

    /// Whether this account's phone is listed in serveradmin.txt.
    pub(crate) fn is_admin_phone(&self, phone: u32) -> bool {
        self.admin.phones.contains(&phone)
    }

    fn admin_say(&mut self, line: &str) {
        self.send_chat(line, ChatType::AdminChat, -1, 0);
    }

    /// handle_player_message for a message starting with '/': /admin <password> makes the player an admin, and an
    /// admin's commands run in the game's order. Every such message is kept from the chat.
    pub(crate) fn admin_message(&mut self, pid: PlayerId, raw: &str) {
        let msg = &raw[..raw.floor_char_boundary(63)];
        let Some(player) = self.players.get_mut(pid.idx()) else { return };

        if let Some(password) = msg.strip_prefix("/admin ") {
            player.admin_tries += 1;

            if player.admin_tries <= 4 && !self.admin.password.is_empty() && password == self.admin.password {
                player.is_admin = true;
                let name = player.username.clone();

                if let Some(c) = self.clients.values_mut().find(|c| c.player_id == pid) {
                    c.admin_visible = true;
                }

                self.admin_say(&format!("{name} admin"));
            }
        }

        if !self.players.get(pid.idx()).is_some_and(|p| p.is_admin) {
            return;
        }

        if msg.starts_with("/hlist") {
            for i in 0..10 {
                let Some(h) = self.humans.get(i) else { continue };

                let line = match h.player.and_then(|p| self.players.get(p.idx()).map(|pl| (p, pl))) {
                    Some((p, pl)) => format!("{i}:{} {}", p.0, pl.username),
                    None => format!("{i}:-1"),
                };

                self.admin_say(&line);
            }
        }

        for (page, cmd) in ["/list1", "/list2", "/list3", "/list4"].into_iter().enumerate() {
            if !msg.starts_with(cmd) {
                continue;
            }

            let conns = self.connection_order();
            let end = if page == 3 { conns.len() } else { (page + 1) * 10 };

            for &conn in conns.iter().take(end).skip(page * 10) {
                let Some(p) = self.clients.get(&conn).and_then(|c| self.players.get(c.player_id.idx())) else { continue };
                let line = format!("{}  {}", p.username, p.phone_number as i32);
                self.admin_say(&line);
            }
        }

        if msg.starts_with("/godmode") {
            self.godmode_command(pid);
        }

        for (cmd, kind) in [
            ("/heli", VehicleKind::Helicopter),
            ("/car", VehicleKind::TownCar),
            ("/beamer", VehicleKind::Beamer),
            ("/pickup", VehicleKind::Pickup),
            ("/turbo", VehicleKind::TurboS),
            ("/van", VehicleKind::Van),
        ] {
            if msg.starts_with(cmd) {
                self.admin_spawn_vehicle(pid, kind);
            }
        }

        if msg.starts_with("/phone") {
            self.admin_phone(pid);
        }

        if msg.starts_with("/arm") {
            self.admin_arm(pid, msg.get(5..).unwrap_or(""));
        }

        if msg.starts_with("/cash")
            && let Some(p) = self.players.get_mut(pid.idx())
        {
            p.money = p.money.wrapping_add(100_000);
        }

        if msg.starts_with("/kill")
            && let Some(h) = self.players.get(pid.idx()).and_then(|p| p.human).and_then(|h| self.humans.get_mut(h))
        {
            h.health.vitality = 0;
        }

        if msg.starts_with("/resetgame") {
            self.admin.reset_requested = true;

            if msg.as_bytes().get(10) == Some(&b' ')
                && let Some(mode) = msg[11..].split_whitespace().next().and_then(|name| crate::gamemode::find(&self.available_modes, name))
            {
                self.admin.next_mode = Some(mode);

                mode.on_admin_select(self);
            }
        }

        if let Some(rest) = msg.strip_prefix("/ban ")
            && !rest.is_empty()
        {
            let phone = leading_int(rest).unwrap_or(0);

            if let Some(a) = self.roster.by_phone(phone) {
                let (id, name) = (a.account_id, a.name.clone());
                self.admin.pending_ban = Some(id);
                self.admin_say(&format!("{name}({phone}) ban?"));
            }
        }

        if let Some(rest) = msg.strip_prefix("/give ")
            && !rest.is_empty()
        {
            let mut nums = numbers(rest);
            let phone = nums.next().unwrap_or(0);
            let amount = nums.next().unwrap_or(0);

            let line = match self.player_by_phone(phone) {
                Some(pid) => {
                    let name = self.players.get(pid.idx()).map(|p| p.username.clone()).unwrap_or_default();

                    if let Some(p) = self.players.get_mut(pid.idx()) {
                        p.money = p.money.wrapping_add(amount);
                    }

                    format!("{name}({phone}) {amount} cash")
                }
                None => format!("{phone} is not playing"),
            };

            self.admin_say(&line);
        }

        if let Some(rest) = msg.strip_prefix("/setrating ")
            && !rest.is_empty()
        {
            let mut nums = numbers(rest);
            let phone = nums.next().unwrap_or(0);
            let rating = nums.next().unwrap_or(0);

            let line = match self.player_by_phone(phone) {
                Some(pid) => {
                    let name = self.players.get(pid.idx()).map(|p| p.username.clone()).unwrap_or_default();

                    if let Some(p) = self.players.get_mut(pid.idx()) {
                        p.crim_rating = rating;
                    }

                    format!("{name}({phone}) {rating} rating")
                }
                None => format!("{phone} is not playing"),
            };

            self.admin_say(&line);
        }
        if let Some(rest) = msg.strip_prefix("/bany ")
            && !rest.is_empty()
            && let Ok(time) = scan_int(rest)
        {
            let time = time.unwrap_or(0);

            if let Some(id) = self.admin.pending_ban {
                let line = self.ban_account(id, time).unwrap_or_else(|| "no such account".to_string());

                self.admin_say(&line);
                self.admin.pending_ban = None;
            }
        }

        if msg.starts_with("/bann") {
            self.admin_say("ban canceled");
            self.admin.pending_ban = None;
        }

        if let Some(rest) = msg.strip_prefix("/unban ")
            && !rest.is_empty()
        {
            let phone = leading_int(rest).unwrap_or(0);
            let account = self.roster.by_phone(phone).map(|a| (a.account_id, a.name.clone()));

            if let Some((id, name)) = account {
                let line = if self.bans.unban(id) { format!("{name}({phone}) unbanned") } else { format!("{name}({phone}) not banned") };

                self.bans.save(std::path::Path::new("bans.txt"));
                self.admin_say(&line);
            }
        }

        if let Some(name) = msg.strip_prefix("/kick ") {
            let target = self
                .connection_order()
                .into_iter()
                .rfind(|c| self.clients.get(c).and_then(|c| self.players.get(c.player_id.idx())).is_some_and(|p| p.username == name));
            if let Some(c) = target.and_then(|t| self.clients.get_mut(&t)) {
                c.timeout = game_seconds(20);
                let ip = c.addr.ip();
                self.admin.kicked.push(ip);
            }
        }

        if let Some(text) = msg.strip_prefix("/message ") {
            self.announce(text);
        }

        if msg.starts_with("/clearkick") {
            self.admin.kicked.clear();
        }
    }

    /// The /godmode chat command: the player's god mode on or off, announced to admins.
    fn godmode_command(&mut self, pid: PlayerId) {
        let Some(p) = self.players.get_mut(pid.idx()) else { return };
        p.god_mode = !p.god_mode;

        let line = if p.god_mode { "godmode on" } else { "godmode off" };
        self.send_chat(line, ChatType::AdminChat, -1, 0);
    }

    /// The connections in the order they joined, standing in for the game's connection array.
    fn connection_order(&self) -> Vec<ConnId> {
        let mut conns: Vec<ConnId> = self.clients.keys().copied().collect();
        conns.sort_by_key(|c| c.0);
        conns
    }

    /// Spawn a vehicle in front of player `pid`
    fn admin_spawn_vehicle(&mut self, pid: PlayerId, kind: VehicleKind) {
        let Some(h) = self.players.get(pid.idx()).and_then(|p| p.human).and_then(|h| self.humans.get(h)) else { return };
        let (p, r) = (h.bones[0].pos, h.bones[0].rot[2]);
        let pos = r * -4.0 + p;

        self.spawn_vehicle(kind, 1, pos, IDENTITY);
    }

    /// /phone: every admin phone hangs up and despawns, then a new one goes in the admin's pocket.
    fn admin_phone(&mut self, pid: PlayerId) {
        for id in self.items.ids() {
            let Some(item) = self.items.get(id) else { continue };
            let Some(p) = item.state.phone().filter(|p| item.item_type == ItemKind::Phone && p.number == 9999) else { continue };

            if let Some(other) = p.connected {
                if let Some(o) = self.items.get_mut(other).and_then(|i| i.state.phone_mut()) {
                    o.connected = None;
                    o.status = PhoneStatus::Idle;
                    o.display_number = 0;
                }

                self.phone_update(other);
            }

            self.mark_item_for_deletion(id);
        }

        let Some(human) = self.players.get(pid.idx()).and_then(|p| p.human) else { return };
        let Some((pos, rot)) = self.humans.get(human).map(|hu| (hu.bones[0].pos, hu.bones[0].rot)) else { return };
        let Some(phone_id) = self.create_item(ItemKind::Phone, pos, None, rot) else { return };

        if let Some(p) = self.items.get_mut(phone_id).and_then(|i| i.state.phone_mut()) {
            p.number = 9999;
        }

        self.link_to_slot(human, phone_id, 6);
    }

    /// /arm [1-5]: a gun with four magazines and two bandages.
    fn admin_arm(&mut self, pid: PlayerId, rest: &str) {
        let Some(h) = self.players.get(pid.idx()).and_then(|p| p.human) else { return };
        let n = if rest.is_empty() { None } else { scan_int(rest).ok().map(|n| n.unwrap_or(0)) };
        let gun = n.and_then(|n| usize::try_from(n - 1).ok()).filter(|&i| i < ARM_GUNS.len()).map_or(ItemKind::Mp5, |i| ARM_GUNS[i]);

        self.give_weapon(h, gun, 4);

        let Some((pos, rot)) = self.humans.get(h).map(|hu| (hu.bones[0].pos, hu.bones[0].rot)) else { return };

        for _ in 0..2 {
            if let Some(id) = self.create_item(ItemKind::Bandage, pos, None, rot) {
                self.link_to_slot(h, id, 6);
            }
        }
    }

    /// Admin actions (logic_playerinteractions types 4 and 5): an account's ban time or criminal rating.
    pub(crate) fn admin_action(&mut self, pid: PlayerId, kind: u8, account: u32, value: u32) {
        if !self.players.get(pid.idx()).is_some_and(|p| p.is_admin) {
            return;
        }

        let (i, value) = (account as i32, value as i32);

        if i < 0 || i as usize >= self.roster.players.len() || value < 0 {
            return;
        }

        let (id, name) = {
            let a = &self.roster.players[i as usize];

            (a.account_id, a.name.clone())
        };
        let line = match kind {
            4 => match self.ban_account(id, value) {
                Some(line) => line,
                None => return,
            },

            5 => {
                for (_, p) in self.players.iter_mut().filter(|(_, p)| p.account_id == id) {
                    p.crim_rating = value;
                }

                format!("{name} {i} rating {value}")
            }
            _ => return,
        };

        self.admin_say(&line);
    }

    fn ban_account(&mut self, account_id: u32, ticks: i32) -> Option<String> {
        let a = self.roster.get(account_id)?;
        let (steam, phone, name) = (a.steam_id, a.phone_number, a.name.clone());
        let seconds = (ticks / game_seconds(1)).max(0) as u64;
        let until = if seconds >= bans::PERMANENT { None } else { Some(bans::now() + seconds) };

        self.bans.ban(account_id, steam, phone, &name, until);
        self.bans.save(std::path::Path::new("bans.txt"));

        Some(match until {
            Some(_) => format!("{name}({}) banned {seconds}s", phone as i32),
            None => format!("{name}({}) banned", phone as i32),
        })
    }

    fn player_by_phone(&self, phone: i32) -> Option<PlayerId> {
        self.players.iter().find(|(_, p)| p.phone_number as i32 == phone && !p.is_bot).map(|(i, _)| PlayerId(i as u32))
    }

    pub(crate) fn is_kicked(&self, ip: IpAddr) -> bool {
        self.admin.kicked.contains(&ip)
    }

    pub(crate) fn connection_timeouts(&mut self) {
        for c in self.clients.values_mut() {
            c.timeout += 1;
        }

        for conn in self.connection_order() {
            let Some(c) = self.clients.get(&conn) else { continue };

            let banned = self.players.get(c.player_id.idx()).is_some_and(|p| self.bans.is_banned(p.account_id, p.steam_id));

            if banned || c.timeout >= game_seconds(30) || self.players.get(c.player_id.idx()).is_none() {
                if let Some(player) = self.players.get(c.player_id.idx()) {
                    println!("[Sim] {} timed out...", player.username);
                }

                self.on_leave(conn);
            }
        }
    }

    pub(crate) fn admin_reset(&mut self) {
        if !self.admin.reset_requested {
            return;
        }

        if let Some(mode) = self.admin.next_mode.filter(|m| m.name() != self.mode.name()) {
            for (_, p) in self.players.iter_mut() {
                p.team = Team::Spectator;
            }

            self.mode = mode;
        }

        self.reset_game();
        self.admin.reset_requested = false;
    }

    /// send_adminpacket for each admin connection whose turn it is.
    pub(crate) fn send_admin_lists(&mut self) {
        let packet = AdminList { rows: vec![None; 256] };
        let tick = self.tick;

        for (index, conn) in self.connection_order().into_iter().enumerate() {
            let Some(c) = self.clients.get(&conn) else { continue };

            if c.admin_visible && tick % 128 == index as u32 % 128 {
                let _ = self.out_tx.send((frame_packet(packet.clone()), c.addr));
            }
        }
    }
}
