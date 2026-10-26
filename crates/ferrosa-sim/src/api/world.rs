use ferrosa_protocol::Team;
use glam::Vec3;

use crate::{PlayerId, sim::Sim, sim::corporations::team_index};

impl Sim {
    pub fn add_npc(&mut self, kind: i32, pos: Vec3, yaw: f32) -> Option<usize> {
        self.create_npc(kind, pos, yaw)
    }

    pub fn set_npc_waypoints(&mut self, npc: usize, waypoints: Vec<Vec3>) {
        if let Some(n) = self.npcs.get_mut(npc).and_then(Option::as_mut) {
            n.waypoints = waypoints;
        }
    }

    pub fn corporation_intel(&self, team: Team) -> i32 {
        team_index(team).map_or(0, |k| self.corp_state[k].intel)
    }

    pub fn set_corporation_intel(&mut self, team: Team, intel: i32) {
        if let Some(k) = team_index(team) {
            self.corp_state[k].intel = intel;
        }
    }

    pub fn set_corporation_prints(&mut self, team: Team, prints: i32) {
        if let Some(k) = team_index(team) {
            self.corp_state[k].prints = prints;
        }
    }

    pub fn player_corp_rating(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.corp_rating)
    }

    pub fn set_player_corp_rating(&mut self, player: PlayerId, rating: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.corp_rating = rating;
        }
    }

    pub fn player_spawn_timer(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.spawn_timer)
    }

    pub fn set_player_spawn_timer(&mut self, player: PlayerId, ticks: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.spawn_timer = ticks;
        }
    }

    pub fn player_vehicles_bought(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.vehicles_bought)
    }

    pub fn set_player_vehicles_bought(&mut self, player: PlayerId, count: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.vehicles_bought = count;
        }
    }

    pub fn set_punish_team_kills(&mut self, on: bool) {
        self.rules.punish_team_kills = on;
    }

    pub fn set_shares_follow_human(&mut self, on: bool) {
        self.rules.shares_follow_human = on;
    }

    pub fn player_crim_rating(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.crim_rating)
    }

    pub fn set_player_crim_rating(&mut self, player: PlayerId, rating: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.crim_rating = rating;
        }
    }

    pub fn send_mission_text(&mut self, team: Team, slot: usize) {
        if let Some(k) = team_index(team) {
            self.push_mission_text(k, slot);
        }
    }

    pub fn alloc_content(&mut self) -> i32 {
        self.fs.alloc_content()
    }

    pub fn content_text(&self, content: i32) -> Option<String> {
        let c = self.fs.content(content)?;

        Some(String::from_utf8_lossy(&c.text).into_owned())
    }

    pub fn set_content_text(&mut self, content: i32, text: &str) {
        if let Some(c) = self.fs.content_mut(content) {
            c.text = text.as_bytes().to_vec();
        }
    }

    pub fn encipher_content(&mut self, content: i32) {
        self.fs.encipher(content);
    }

    pub fn volume_add_file(&mut self, volume: i32, name: &str, content: i32) -> i32 {
        self.fs.add_file(volume, 0, name, content)
    }

    pub fn volume_exists(&self, volume: i32) -> bool {
        self.fs.volume(volume).is_some()
    }

    pub fn volume_swap_files(&mut self, volume: i32, a: usize, b: usize) {
        let Some(v) = self.fs.volume_mut(volume) else { return };

        if a < v.files.len() && b < v.files.len() {
            v.files.swap(a, b);
        }
    }

    pub fn place_team_memo(&mut self, team: Team, text: &str) {
        if let Some(k) = team_index(team) {
            self.place_corporation_memo(k, text.as_bytes());
        }
    }

    pub fn vehicle_owner(&self, vehicle: usize) -> Option<PlayerId> {
        let owner = self.vehicles.get(vehicle).map_or(-1, |v| v.owner);

        u32::try_from(owner).ok().map(PlayerId)
    }

    pub fn vehicle_traffic_car(&self, vehicle: usize) -> Option<usize> {
        self.traffic.cars.iter().position(|c| c.vehicle == vehicle as i32)
    }

    pub fn add_mission_bot(&mut self, expiry: i32, pos: Vec3, rot: ferrosa_physics::RotMatrix, vehicle: usize, seat: usize) -> Option<(PlayerId, Option<usize>)> {
        self.create_mission_bot(expiry, None, pos, rot, vehicle, seat)
    }

    pub fn add_guards(&mut self, expiry: i32, pos: Vec3, rot: ferrosa_physics::RotMatrix, vehicle: usize) {
        self.create_guards(expiry, None, pos, rot, vehicle);
    }

    pub fn traffic_car_route(&self, car: usize) -> Vec<i32> {
        let Some(c) = self.traffic.cars.get(car) else { return Vec::new() };
        let len = (c.route_len.max(0) as usize).min(c.route.len());

        c.route[..len].iter().map(|s| s.street).collect()
    }

    pub fn tick_count(&self) -> u32 {
        self.tick
    }

    pub fn file_content(&self, volume: i32, file: usize) -> i32 {
        self.fs.volume(volume).and_then(|v| v.files.get(file)).map_or(-1, |f| f.data)
    }

    pub fn player_account_id(&self, player: PlayerId) -> Option<u32> {
        self.players.get(player.idx()).map(|p| p.account_id).filter(|&a| a != u32::MAX)
    }

    pub fn account_name(&self, account_id: u32) -> Option<String> {
        self.roster.get(account_id).map(|p| p.name.clone())
    }

    pub fn bullets_fired(&self) -> u32 {
        self.stats.bullets
    }

    pub fn missions_settled(&self) -> u32 {
        self.stats.missions
    }

    pub fn send_stock_prices(&mut self) {
        let e = crate::sim::economy::stock_event(&self.corporations, self.tick);
        self.events.push(e);
    }

    pub fn human_vehicle_seat(&self, human: usize) -> Option<u32> {
        let h = self.humans.get(human)?;

        h.vehicle.map(|_| h.seat as u32)
    }

    pub fn human_yaw(&self, human: usize) -> f32 {
        self.humans.get(human).map_or(0.0, |h| h.bones[0].angles.x)
    }

    pub fn human_max_stamina(&self, human: usize) -> i32 {
        self.humans.get(human).map_or(0, |h| h.max_stamina)
    }

    pub fn set_human_max_stamina(&mut self, human: usize, stamina: i32) {
        if let Some(h) = self.humans.get_mut(human) {
            h.max_stamina = stamina;
        }
    }

    pub fn item_despawn_time(&self, item: usize) -> i32 {
        self.items.get(item).map_or(0, |i| i.despawn_time)
    }

    pub fn set_item_despawn_time(&mut self, item: usize, ticks: i32) {
        if let Some(i) = self.items.get_mut(item) {
            i.despawn_time = ticks;
        }
    }

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

    pub fn player_bot_deadline(&self, player: PlayerId) -> i32 {
        self.players.get(player.idx()).map_or(0, |p| p.bot_deadline)
    }

    pub fn set_player_bot_deadline(&mut self, player: PlayerId, clock: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            p.bot_deadline = clock;
        }
    }

    pub fn write_item_memo(&mut self, item: usize, text: &str) {
        self.write_memo(item, text.as_bytes());
    }

    pub fn settle_mission(&mut self, team: Team, slot: usize, success: bool, cash: i32) {
        if let Some(k) = team_index(team) {
            self.settle_mission_result(k, slot, success, cash);
        }
    }

    pub fn despawn_missions(&mut self, id: i32) {
        self.despawn_missions_by_id(id);
    }

    pub fn set_player_corp_money(&mut self, player: PlayerId, money: i32, credit: i32) {
        if let Some(p) = self.players.get_mut(player.idx()) {
            (p.corp_money, p.corp_credit) = (money, credit);
        }
    }

    pub fn set_radio_channel(&mut self, item: usize, channel: i32) -> bool {
        match self.items.get_mut(item).map(|i| &mut i.state) {
            Some(crate::items::state::ItemState::Radio { channel: c, .. }) => {
                *c = channel;
                true
            }
            _ => false,
        }
    }

    pub fn set_vehicle_train_index(&mut self, vehicle: usize, index: i32) {
        if let Some(v) = self.vehicles.get_mut(vehicle) {
            v.train_index = index;
        }
    }

    pub fn disconnect_team_phones(&mut self, team: Team) {
        if let Some(k) = team_index(team) {
            self.disconnect_phone(k);
        }
    }
}
