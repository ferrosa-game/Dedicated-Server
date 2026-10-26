use ferrosa_wit::{PlayerId, Team, server};

use crate::{World, state::CORPORATIONS};

/// A world account is worth its money plus its shares at 95% of the price.
const SALE_SHARE: f32 = 0.95;
/// Every world save adds 5 to a player's play time.
const PLAY_TIME_STEP: u32 = 5;
/// Every 18000 a human's stamina ceiling drops by 2 (8 drops to 7, and 7 and below stay).
const STAMINA_FLOOR: i32 = 7;
const STAMINA_DROP: i32 = 2;
/// Every 54000 accounts lose 16 criminal rating, and a criminal player's own rating moves 8 onto their account and
/// drops by 1.
const ACCOUNT_CRIME_DECAY: u32 = 16;
const ACCOUNT_CRIME_GAIN: u32 = 8;
/// A death waits out a respawn timer that grows with criminal rating.
const WORLD_RESPAWN_BASE: i32 = 600;
const WORLD_RESPAWN_PER_CRIME: i32 = 6;

impl World {
    /// The world account bookkeeping of player_simulation: every player's account holds their money (with their
    /// shares sold at 95%) and their ratings.
    pub fn world_account_sync(&mut self) {
        for p in server::players() {
            let Some(mut a) = p.account() else { continue };

            let worth = match crate::state::World::team_index(p.team()).and_then(|_| server::get_corporation(p.team())) {
                Some(c) => (p.money() as f32 + p.stocks() as f32 * (SALE_SHARE * c.price())) as i32,
                None => p.money(),
            };

            a.money = worth.max(0) as u32;
            a.corp_rating = self.state.corp_rating(p.id()).max(0) as u32;
            a.crim_rating = p.crim_rating().max(0) as u32;
            p.set_account(a);
            p.set_account_spawn_timer(self.state.spawn_timer(p.id()));
        }
    }

    /// Every 18000: stamina ceilings drop, the shops restock, and the accounts, play times and stats are saved with
    /// the share prices sent.
    pub fn world_save_tick(&mut self) {
        for h in server::humans() {
            let max = h.max_stamina();

            if max > STAMINA_FLOOR {
                h.set_max_stamina(if max == STAMINA_FLOOR + 1 { STAMINA_FLOOR } else { max - STAMINA_DROP });
            }
        }

        server::save_accounts();

        for p in server::players() {
            if let Some(mut a) = p.account() {
                a.play_time += PLAY_TIME_STEP;
                p.set_account(a);
            }
        }

        server::save_stats();
        server::send_stock_prices();
    }

    /// The account side of the crime pass: accounts lose 16 criminal rating, and a criminal player moves 8 of theirs
    /// onto their account and drops by 1.
    pub fn account_crime_tick(&mut self) {
        for p in server::players() {
            let crim = p.crim_rating();

            if crim <= 0 {
                if let Some(mut a) = p.account()
                    && a.crim_rating > 0
                {
                    a.crim_rating = a.crim_rating.saturating_sub(ACCOUNT_CRIME_DECAY);
                    p.set_account(a);
                }

                continue;
            }

            if let Some(mut a) = p.account() {
                a.crim_rating += ACCOUNT_CRIME_GAIN;
                p.set_account(a);
            }

            p.set_crim_rating(crim - 1);
        }
    }

    /// A death in world mode: the player waits out a timer that grows with their criminal rating, spectating unless
    /// respawnteam is set.
    pub fn world_on_death(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };

        if !self.state.cfg.respawnteam {
            self.set_world_team(pid, Team::Spectator);
        }

        let wait = p.crim_rating() * WORLD_RESPAWN_PER_CRIME + WORLD_RESPAWN_BASE;
        self.state.spawn_timers.insert(pid, wait);

        if self.state.cfg.shares_follow_human {
            p.sell_all_stocks();
        }
    }

    /// update_corp_player_ratings: a corporation pays its members a tenth of its money at the end of the day, the
    /// manager half of it, and its bank goes back to nothing.
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
            let rating = self.state.corp_rating(id) + pay;

            self.state.ratings.insert(id, rating);
        }

        self.state.corps[k].money = 0;
    }

    /// The end of the day: the corporations pay their players, everyone's whereabouts are kept, the accounts are
    /// written and the world starts over.
    pub fn end_world_day(&mut self) {
        for k in 0..CORPORATIONS.len() {
            self.update_corp_player_ratings(k);
        }

        self.state.saved_vehicles.clear();
        self.save_inventories();
        self.world_account_sync();
        server::save_accounts();
        server::request_reset();
    }

    /// Every player's things and whereabouts kept for the next day.
    pub fn save_inventories(&mut self) {
        self.state.saved_bodies.clear();

        for p in server::players() {
            self.save_inventory(p.id());

            if let Some(h) = p.human() {
                let vehicle = h.vehicle().map(|v| {
                    let id = v.id();
                    self.state.saved_vehicles.push(crate::state::SavedVehicle { old: id, new: None, kind: v.kind(), colour: v.colour(), pos: v.pos(), rot: v.rot() });

                    id
                });

                let body = crate::state::SavedBody { pos: h.pos(), yaw: h.yaw(), vehicle, seat: h.vehicle_seat().unwrap_or(0) };
                self.state.saved_bodies.insert(p.id(), body);
            }
        }
    }

    /// One player's items kept for the next day: what each hand and pocket holds.
    pub fn save_inventory(&mut self, pid: PlayerId) {
        let Some(human) = server::get_player(pid).and_then(|p| p.human()) else { return };
        let mut saved: Vec<crate::state::SavedItem> = Vec::new();

        for slot in 0..7u32 {
            for item in human.slot(slot) {
                let magazine = item.children().first().map(|m| (m.kind(), m.uses()));

                saved.push(crate::state::SavedItem { slot, kind: item.kind(), uses: item.uses(), magazine, cash: item.cash() });
            }
        }

        self.state.saved_items.insert(pid, saved);
    }

    /// A rejoining player's money and ratings taken back off their account.
    pub fn restore_account(&mut self, pid: PlayerId) {
        let Some(p) = server::get_player(pid) else { return };

        let Some(a) = p.account() else {
            p.set_money(self.state.cfg.startcash);

            return;
        };

        p.set_money(a.money as i32);
        p.set_crim_rating(a.crim_rating as i32);
        self.state.ratings.insert(pid, a.corp_rating as i32);
        self.state.spawn_timers.insert(pid, p.account_spawn_timer());
    }

    /// Their human's things put back as the day ended, and their whereabouts with them.
    pub fn restore_inventory(&mut self, pid: PlayerId) {
        let Some(human) = server::get_player(pid).and_then(|p| p.human()) else { return };
        let Some(saved) = self.state.saved_items.remove(&pid) else { return };

        for s in saved {
            let Some(item) = human.give_item(s.kind, s.slot, s.slot + 1) else { continue };

            item.set_uses(s.uses);

            if let Some(cash) = &s.cash {
                item.set_cash(cash);
            }

            if let Some((kind, uses)) = s.magazine
                && let Some(mag) = server::spawn_item_by_id(kind, human.pos(), ferrosa_wit::Mat3::IDENTITY)
            {
                mag.set_uses(uses);
                item.attach(&mag);
            }
        }
    }

    /// respawn_world: the saved vehicles come back where they were, and every player gets their human back where
    /// they left it with what they held.
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
