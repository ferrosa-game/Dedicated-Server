use ferrosa_wit::{ItemKind, Mat3, PlayerId, Team, game_seconds, server};

use crate::{
    World,
    state::{CORPORATIONS, SavedBody, SavedItem, SavedVehicle},
};

impl World {
    pub fn world_account_sync(&mut self) {
        for p in server::players() {
            self.save_player_account(p.id());
        }
    }

    pub fn save_player_account(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };
        let Some(id) = p.account_id() else { return };

        let worth = match crate::state::World::team_index(p.team()).and_then(|_| server::get_corporation(p.team())) {
            Some(c) => (p.money() as f32 + p.stocks() as f32 * (c.price() * 0.95)) as i32,
            None => p.money(),
        };
        let name = server::account_name(id).unwrap_or_default();
        let a = self.state.accounts.entry(id);

        a.money = worth.max(0);
        a.corp_rating = p.corp_rating().max(0);
        a.crim_rating = p.crim_rating().max(0);
        a.spawn_timer = p.spawn_timer();
        a.name = name;
    }

    pub fn world_save_tick(&mut self) {
        for h in server::humans() {
            let max = h.max_stamina();

            if max > 7 {
                h.set_max_stamina(if max == 8 { 7 } else { max - 2 });
            }
        }

        self.world_account_sync();

        for p in server::players() {
            if let Some(id) = p.account_id() {
                self.state.accounts.entry(id).play_time += 5;
            }
        }

        self.state.accounts.save();
        self.save_stats();
        server::send_stock_prices();
    }

    pub fn save_stats(&self) {
        let accounts = &self.state.accounts;
        let total = accounts.iter().fold(0u32, |t, (_, a)| t.wrapping_add(a.play_time / 10));
        let average = if accounts.len() == 0 { 0 } else { total / accounts.len() as u32 };
        let mut out = format!(
            "numofmissions={}
            numofbullets={}
            users={}
            totaltime={}
            averagetime={}",
            server::missions_settled() as i32,
            server::bullets_fired() as i32,
            accounts.len(),
            total as i32,
            average as i32
        );

        for (i, (id, a)) in accounts.iter().enumerate() {
            out += &format!(
                "{i}:{}:{} ({})",
                a.name,
                (a.play_time / 10) as i32,
                *id as i32
            );
        }

        if let Err(e) = std::fs::write("stats.txt", out) {
            eprintln!("[world] Could not save stats: {e}");
        }
    }

    pub fn account_crime_tick(&mut self) {
        for p in server::players() {
            let crim = p.crim_rating();
            let Some(id) = p.account_id() else { continue };

            if crim <= 0 {
                let a = self.state.accounts.entry(id);

                if a.crim_rating > 0 {
                    a.crim_rating = (a.crim_rating - 16).max(0);
                }

                continue;
            }

            if p.human().is_some() {
                self.state.accounts.entry(id).crim_rating += 8;
            }

            p.set_crim_rating(crim - 1);
        }
    }

    pub fn world_on_death(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };

        if !self.state.cfg.respawnteam {
            self.set_world_team(pid, Team::Spectator);
        }

        let wait = p.crim_rating() * 6 + game_seconds(10);
        p.set_spawn_timer(wait);

        if self.state.cfg.shares_follow_human {
            p.sell_all_stocks();
        }
    }

    pub fn update_corp_player_ratings(&mut self, k: usize) {
        let team = CORPORATIONS[k];
        let members: Vec<_> = server::players().into_iter().filter(|p| p.team() == team).collect();

        if members.is_empty() {
            return;
        }

        let tenth = self.state.corps[k].money / 10;
        let share = if tenth < 0 { tenth * 2 } else { tenth };
        let count = members.len() as i32;
        let (manager_share, member_share) = if count == 1 { (share, share) } else { (share / 2, share / 2 / (count - 1)) };
        let manager = self.state.corps[k].manager;

        for p in members {
            let id = p.id();
            let pay = if manager == Some(id) { manager_share } else { member_share };
            p.set_corp_rating(p.corp_rating() + pay);
            let _ = id;
        }

        self.state.corps[k].money = 0;
    }

    pub fn end_world_day(&mut self) {
        for k in 0..CORPORATIONS.len() {
            self.update_corp_player_ratings(k);
        }

        self.state.saved_vehicles.clear();
        self.save_inventories();
        self.world_account_sync();
        self.state.accounts.save();
        server::request_reset();
    }

    pub fn save_inventories(&mut self) {
        self.state.saved_bodies.clear();

        for p in server::players() {
            self.save_inventory(p.id());

            if let Some(h) = p.human() {
                let vehicle = h.vehicle().map(|v| {
                    let id = v.id();

                    if !self.state.saved_vehicles.iter().any(|s| s.old == id) {
                        self.state.saved_vehicles.push(SavedVehicle { old: id, new: None, kind: v.kind(), colour: v.colour(), pos: v.pos(), rot: v.rot() });
                    }

                    id
                });

                let body = SavedBody { pos: h.pos(), yaw: h.yaw(), vehicle, seat: h.vehicle_seat().unwrap_or(0) };
                self.state.saved_bodies.insert(p.id(), body);
            }
        }
    }

    pub fn save_inventory(&mut self, pid: PlayerId) {
        let Some(human) = server::get_player(pid).and_then(|p| p.human()) else { return };
        let mut saved: Vec<SavedItem> = Vec::new();

        for slot in 0..7 {
            for item in human.slot(slot) {
                let kind = item.kind();

                if kind <= ItemKind::Bandage as u32 {
                    let left = item.uses();
                    let mut magazine = None;

                    if let Some(mag) = item.children().first() {
                        let rounds = mag.uses();
                        let full = server::item_magazine_ammo(mag.kind()).unwrap_or(0);

                        // A chambered round goes back into the magazine, as the game puts it back.
                        magazine = Some((mag.kind(), if left > 0 && rounds <= full { rounds + 1 } else { rounds }));
                    }

                    saved.push(SavedItem { slot, kind, uses: left, magazine, cash: None, key_vehicle: None });
                } else if kind == ItemKind::Key as u32 {
                    if let Some(car) = item.key_vehicle().filter(|v| v.health() > 0) {
                        let id = car.id();
                        self.state.saved_vehicles.push(SavedVehicle {
                            old: id,
                            new: None,
                            kind: car.kind(),
                            colour: car.colour(),
                            pos: car.pos(),
                            rot: car.rot(),
                        });

                        saved.push(SavedItem { slot, kind, uses: 0, magazine: None, cash: None, key_vehicle: Some(id) });
                    }
                } else if let Some(cash) = item.cash() {
                    saved.push(SavedItem { slot, kind, uses: 0, magazine: None, cash: Some(cash), key_vehicle: None });
                }
            }
        }

        self.state.saved_items.insert(pid, saved);
    }

    pub fn restore_account(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };
        let Some(id) = p.account_id() else { return };
        let (startcash, mincash) = (self.state.cfg.startcash, self.state.cfg.mincash);
        let a = self.state.accounts.open(id, startcash);

        p.set_money(a.money.max(mincash));
        p.set_corp_rating(a.corp_rating.max(0));
        p.set_crim_rating(a.crim_rating);
        p.set_spawn_timer(a.spawn_timer);
    }

    pub fn restore_inventory(&mut self, pid: PlayerId) {
        let Some(human) = server::get_player(pid).and_then(|p| p.human()) else { return };
        let Some(saved) = self.state.saved_items.remove(&pid) else { return };

        for s in saved {
            let Some(item) = human.give_item(s.kind, s.slot, s.slot + 1) else { continue };

            item.set_uses(s.uses);

            if let Some(cash) = &s.cash {
                item.set_cash(cash);
            }

            if let Some(old) = s.key_vehicle {
                let car = self.state.saved_vehicles.iter().find(|v| v.old == old).and_then(|v| v.new).and_then(server::get_vehicle);

                item.set_key_vehicle(car.as_ref());
            }

            if let Some((kind, uses)) = s.magazine
                && let Some(mag) = server::spawn_item_by_id(kind, human.pos(), Mat3::IDENTITY)
            {
                mag.set_uses(uses);
                item.attach(&mag);
            }
        }
    }

    pub fn respawn_world(&mut self) {
        for k in 0..self.state.saved_vehicles.len() {
            let sv = &self.state.saved_vehicles[k];
            let spawned = server::spawn_vehicle_by_id(sv.kind, sv.colour, sv.pos, sv.rot);

            self.state.saved_vehicles[k].new = spawned.map(|v| v.id());
        }

        for p in server::players() {
            let Some(body) = self.state.saved_bodies.remove(&p.id()) else { continue };
            let Some(human) = server::spawn_human(body.pos, ferrosa_wit::Mat3::from_yaw(body.yaw), Some(p.id())) else { continue };

            p.set_human(&human);
            human.set_view_yaw(body.yaw);
            self.restore_inventory(p.id());

            if let Some(old) = body.vehicle
                && let Some(new) = self.state.saved_vehicles.iter().find(|s| s.old == old).and_then(|s| s.new)
                && let Some(v) = server::get_vehicle(new)
            {
                human.enter_vehicle(&v, body.seat);
            }
        }
    }
}
