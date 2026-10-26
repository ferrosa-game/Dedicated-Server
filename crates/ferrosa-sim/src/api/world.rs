use ferrosa_protocol::Team;
use glam::Vec3;

use crate::{PlayerId, sim::Sim, sim::corporations::team_index};

/// The rates a mode sets for the server's own criminal-rating scoring.
#[derive(Clone, Copy, Debug, Default)]
pub struct CrimeRates {
    pub civ_civ: i32,
    pub civ_team: i32,
    pub team_civ: i32,
    pub team_team: i32,
    pub team_team_in_base: i32,
    pub kick: i32,
}

impl Sim {
    /// A new NPC of `kind` standing at `pos` facing `yaw`, which the server's own NPC code then runs; none once all
    /// 1024 slots are taken.
    pub fn add_npc(&mut self, kind: i32, pos: Vec3, yaw: f32) -> Option<usize> {
        self.create_npc(kind, pos, yaw)
    }

    /// The route an NPC walks, from the first point, round and round.
    pub fn set_npc_waypoints(&mut self, npc: usize, waypoints: Vec<Vec3>) {
        if let Some(n) = self.npcs.get_mut(npc).and_then(Option::as_mut) {
            n.waypoints = waypoints;
        }
    }

    /// The intel a corporation can ask for: 1 to 8 can request, 0 requested, 11 faxed, others none.
    pub fn corporation_intel(&self, team: Team) -> i32 {
        team_index(team).map_or(0, |k| self.corp_state[k].intel)
    }

    pub fn set_corporation_intel(&mut self, team: Team, intel: i32) {
        if let Some(k) = team_index(team) {
            self.corp_state[k].intel = intel;
        }
    }

    /// How many intel screens the corporation has printed; the server stops it at four.
    pub fn set_corporation_prints(&mut self, team: Team, prints: i32) {
        if let Some(k) = team_index(team) {
            self.corp_state[k].prints = prints;
        }
    }

    /// A player's criminal rating, which the server's own scoring raises when they hurt people.
    pub fn player_crim_rating(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.crim_rating)
    }

    pub fn set_player_crim_rating(&mut self, player: PlayerId, rating: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.crim_rating = rating;
        }
    }

    /// Sends a mission's result text to the corporation's clients, which their MISSION.EXE shows.
    pub fn send_mission_text(&mut self, team: Team, slot: usize) {
        if let Some(k) = team_index(team) {
            self.push_mission_text(k, slot);
        }
    }

    /// A new empty file content; -1 once all 8192 are taken. A content is the text behind a file, and several files
    /// can share one.
    pub fn alloc_content(&mut self) -> i32 {
        self.fs.alloc_content()
    }

    /// A content's text, read through its cipher.
    pub fn content_text(&self, content: i32) -> Option<String> {
        let c = self.fs.content(content)?;

        Some(String::from_utf8_lossy(&c.text).into_owned())
    }

    pub fn set_content_text(&mut self, content: i32, text: &str) {
        if let Some(c) = self.fs.content_mut(content) {
            c.text = text.as_bytes().to_vec();
        }
    }

    /// Enciphers a content with a fresh cipher, so DECRYPT must be run on it in game.
    pub fn encipher_content(&mut self, content: i32) {
        self.fs.encipher(content);
    }

    /// A file on `volume` named `name` reading `content`, which may already be another file's. The file's number, or
    /// -1 once the volume is full.
    pub fn volume_add_file(&mut self, volume: i32, name: &str, content: i32) -> i32 {
        self.fs.add_file(volume, 0, name, content)
    }

    /// Whether a volume number is one that exists.
    pub fn volume_exists(&self, volume: i32) -> bool {
        self.fs.volume(volume).is_some()
    }

    /// Swaps two of a volume's files, as the decoy documents are shuffled.
    pub fn volume_swap_files(&mut self, volume: i32, a: usize, b: usize) {
        let Some(v) = self.fs.volume_mut(volume) else { return };

        if a < v.files.len() && b < v.files.len() {
            v.files.swap(a, b);
        }
    }

    /// A memo on the corporation's table, headed with its name and the time.
    pub fn place_team_memo(&mut self, team: Team, text: &str) {
        if let Some(k) = team_index(team) {
            self.place_corporation_memo(k, text.as_bytes());
        }
    }

    /// The traffic car a vehicle is, if it is one: the number traffic-car-route takes.
    pub fn vehicle_traffic_car(&self, vehicle: usize) -> Option<usize> {
        self.traffic.cars.iter().position(|c| c.vehicle == vehicle as i32)
    }

    /// A mission bot seated in `vehicle`, which leaves at `expiry`: the player and its human.
    pub fn add_mission_bot(&mut self, expiry: i32, pos: Vec3, rot: ferrosa_physics::RotMatrix, vehicle: usize, seat: usize) -> Option<(PlayerId, Option<usize>)> {
        self.create_mission_bot(expiry, None, pos, rot, vehicle, seat)
    }

    /// A vehicle's guards: one in the front passenger seat, each back seat taken at even odds, all armed.
    pub fn add_guards(&mut self, expiry: i32, pos: Vec3, rot: ferrosa_physics::RotMatrix, vehicle: usize) {
        self.create_guards(expiry, None, pos, rot, vehicle);
    }

    /// The streets a traffic car is driving, in order.
    pub fn traffic_car_route(&self, car: usize) -> Vec<i32> {
        let Some(c) = self.traffic.cars.get(car) else { return Vec::new() };
        let len = (c.route_len.max(0) as usize).min(c.route.len());

        c.route[..len].iter().map(|s| s.street).collect()
    }

    /// The server's tick count, which counts on from the reset.
    pub fn tick_count(&self) -> u32 {
        self.tick
    }

    /// The content a file reads, which several files can share.
    pub fn file_content(&self, volume: i32, file: usize) -> i32 {
        self.fs.volume(volume).and_then(|v| v.files.get(file)).map_or(-1, |f| f.data)
    }

    /// A player's saved account, which the server keeps across days and restores when they rejoin.
    pub fn player_account(&self, player: PlayerId) -> Option<(u32, u32, u32, u32)> {
        let id = self.players.get(player.idx()).map(|p| p.account_id).filter(|&a| a != u32::MAX)?;
        let a = self.saved_accounts.players.iter().find(|a| a.account_id == id)?;

        Some((a.money, a.corp_rating, a.crim_rating, a.play_time))
    }

    /// Sets what a player's account holds. A mode that keeps money and ratings across days writes them here.
    pub fn set_player_account(&mut self, player: PlayerId, money: u32, corp_rating: u32, crim_rating: u32, play_time: u32) {
        let Some(id) = self.players.get(player.idx()).map(|p| p.account_id).filter(|&a| a != u32::MAX) else { return };

        if let Some(a) = self.saved_accounts.get_player_data(id) {
            (a.money, a.corp_rating, a.crim_rating, a.play_time) = (money, corp_rating, crim_rating, play_time);
        }
    }

    /// How long a player must wait before spawning again, which the server keeps with their account.
    pub fn player_account_spawn_timer(&self, player: PlayerId) -> i32 {
        let Some(id) = self.players.get(player.idx()).map(|p| p.account_id) else { return 0 };

        self.account_spawn_timers.get(&id).copied().unwrap_or(0)
    }

    pub fn set_player_account_spawn_timer(&mut self, player: PlayerId, ticks: i32) {
        let Some(id) = self.players.get(player.idx()).map(|p| p.account_id) else { return };

        self.account_spawn_timers.insert(id, ticks);
    }

    /// Writes the accounts out, as world mode does at every save.
    pub fn save_account_file(&mut self) {
        self.save_accounts();
    }

    /// Writes the server's stats out.
    pub fn save_stats_file(&mut self) {
        self.save_stats();
    }

    /// Sends every client the corporations' share prices.
    pub fn send_stock_prices(&mut self) {
        let e = crate::sim::economy::stock_event(&self.corporations, self.tick);
        self.events.push(e);
    }

    /// Which of a vehicle's seats a human is in, 0 the driver's; none for one on foot.
    pub fn human_vehicle_seat(&self, human: usize) -> Option<u32> {
        let h = self.humans.get(human)?;

        h.vehicle.map(|_| h.seat as u32)
    }

    /// The way a human's body faces along the ground, in radians.
    pub fn human_yaw(&self, human: usize) -> f32 {
        self.humans.get(human).map_or(0.0, |h| h.bones[0].angles.x)
    }

    /// A human's stamina ceiling, which world mode wears down over a day.
    pub fn human_max_stamina(&self, human: usize) -> i32 {
        self.humans.get(human).map_or(0, |h| h.max_stamina)
    }

    pub fn set_human_max_stamina(&mut self, human: usize, stamina: i32) {
        if let Some(h) = self.humans.get_mut(human) {
            h.max_stamina = stamina;
        }
    }

    /// How long an item has before it despawns; 0 for one that never does.
    pub fn item_despawn_time(&self, item: usize) -> i32 {
        self.items.get(item).map_or(0, |i| i.despawn_time)
    }

    pub fn set_item_despawn_time(&mut self, item: usize, ticks: i32) {
        if let Some(i) = self.items.get_mut(item) {
            i.despawn_time = ticks;
        }
    }

    /// A bot's human let go of: the traffic car it drives goes back to the traffic, what it holds stops despawning,
    /// and the human stays in the world with nobody controlling it.
    pub fn release_bot_human(&mut self, human: usize) {
        let Some(hu) = self.humans.get(human) else { return };

        if let Some(c) =
            hu.vehicle.and_then(|v| self.vehicles.get(v)).and_then(|v| usize::try_from(v.traffic_car).ok()).and_then(|t| self.traffic.cars.get_mut(t))
        {
            c.is_bot = crate::traffic::DRIVEN;
            c.is_aggressive = 0;
        }

        let held: Vec<usize> = hu.inventory.iter().flat_map(|s| s.items[..s.count.max(0) as usize].iter().map(|&i| i as usize)).collect();

        for id in held {
            if let Some(&child) = self.items.get(id).and_then(|i| i.children.first()) {
                if let Some(c) = self.items.get_mut(child) {
                    c.despawn_time = 0;
                }

                crate::items::remove_link(&mut self.items, child, id);
            }

            if let Some(i) = self.items.get_mut(id) {
                i.despawn_time = 0;
            }
        }

        if let Some(hu) = self.humans.get_mut(human) {
            hu.despawn_ticks = 0;
            hu.player = None;
        }
    }

    /// The world clock a bot leaves at; 0 for one that stays.
    pub fn player_bot_deadline(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.bot_deadline)
    }

    pub fn set_player_bot_deadline(&mut self, player: PlayerId, clock: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.bot_deadline = clock;
        }
    }

    /// The text a paper or memo shows, as the game's own memos are written.
    pub fn write_item_memo(&mut self, item: usize, text: &str) {
        self.write_memo(item, text.as_bytes());
    }

    /// settle_mission_result: a corporation's mission slot decided, paying `cash` and settling its rating.
    pub fn settle_mission(&mut self, team: Team, slot: usize, success: bool, cash: i32) {
        if let Some(k) = team_index(team) {
            self.settle_mission_result(k, slot, success, cash);
        }
    }

    /// The items of a deal's missions taken out of the world.
    pub fn despawn_missions(&mut self, id: i32) {
        self.despawn_missions_by_id(id);
    }

    /// The corporation money and credit a player's client is shown. A mode that runs corporations sets these; the
    /// server only sends them on.
    pub fn set_player_corp_money(&mut self, player: PlayerId, money: i32, credit: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            (p.corp_money, p.corp_credit) = (money, credit);
        }
    }

    /// The channel a walkie-talkie talks on; false if the item isn't one.
    pub fn set_radio_channel(&mut self, item: usize, channel: i32) -> bool {
        match self.items.get_mut(item).map(|i| &mut i.state) {
            Some(crate::items::state::ItemState::Radio { channel: c, .. }) => {
                *c = channel;
                true
            }
            _ => false,
        }
    }

    /// Hurting people raises criminal ratings by these rates, and the kick applies; none turns it off.
    pub fn set_crime(&mut self, rates: Option<CrimeRates>) {
        let Some(r) = rates else {
            self.rules.crime = false;

            return;
        };

        self.rules.crime = true;
        self.rules.crimecivciv = r.civ_civ;
        self.rules.crimecivteam = r.civ_team;
        self.rules.crimeteamciv = r.team_civ;
        self.rules.crimeteamteam = r.team_team;
        self.rules.crimeteamteaminbase = r.team_team_in_base;
        self.rules.crimekick = r.kick;
    }

    /// Which of the track's stations a train runs from.
    pub fn set_vehicle_train_index(&mut self, vehicle: usize, index: i32) {
        if let Some(v) = self.vehicles.get_mut(vehicle) {
            v.train_index = index;
        }
    }

    /// Hangs up both of a corporation's phones, wherever they are.
    pub fn disconnect_team_phones(&mut self, team: Team) {
        if let Some(k) = team_index(team) {
            self.disconnect_phone(k);
        }
    }
}
