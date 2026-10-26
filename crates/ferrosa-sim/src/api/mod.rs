use ferrosa_protocol::clientbound::game::{
    GameState,
    events::{
        Event, ServerEvent,
        sound::{EventSound, Sound},
    },
};
use glam::Vec3;

use crate::{
    PlayerId,
    sim::{Sim, economy::stock_event},
    time::{WORLD_HOUR, WORLD_MINUTE},
    world::trace::line_intersect_level,
};

mod buildings;
mod chat;
mod computers;
mod corporations;
mod humans;
mod items;
mod physics;
mod players;
mod vehicles;
pub mod world;

#[derive(Default)]
pub(crate) struct PendingUpdates {
    players: Vec<PlayerId>,
    rounds: Vec<PlayerId>,
    stock_prices: bool,
}

fn mark(list: &mut Vec<PlayerId>, player: PlayerId) {
    if !list.contains(&player) {
        list.push(player);
    }
}

impl Sim {
    pub fn gamestate(&self) -> GameState {
        self.gamestate
    }

    pub fn set_gamestate(&mut self, state: GameState) {
        self.gamestate = state;
    }

    pub fn mode_name(&self) -> &'static str {
        self.mode.name()
    }

    pub fn flush_updates(&mut self) {
        let updates = std::mem::take(&mut self.updates);
        let tick = self.tick;

        for player in updates.players {
            self.update_player_event(player);
        }

        for player in updates.rounds {
            if let Some(p) = self.players.get(player.idx()) {
                self.events.push(p.make_update_financials_event(tick));
            }
        }

        if updates.stock_prices {
            self.events.push(stock_event(&self.corporations, tick));
        }
    }

    pub fn game_timer(&self) -> i32 {
        self.game_timer
    }

    pub fn set_game_timer(&mut self, ticks: i32) {
        self.game_timer = ticks;
    }

    pub fn world_time(&self) -> i32 {
        self.world_time
    }

    pub fn set_world_time(&mut self, time: i32) {
        self.world_time = time.rem_euclid(24 * WORLD_HOUR);
    }

    pub fn set_world_time_clock(&mut self, hours: i32, minutes: i32) {
        self.set_world_time((hours * 60 + minutes) * WORLD_MINUTE);
    }

    pub fn set_week(&mut self, weekly: bool, weekday: u8) {
        self.week = Some((weekly, weekday));
    }

    pub fn set_traffic(&mut self, cars: u32, parked: bool) {
        self.do_traffic = Some(self.do_traffic.unwrap_or(0) + cars);
        self.rules.parked_cars = parked;
    }

    pub fn set_keep_bodies(&mut self, keep: bool) {
        self.rules.keep_bodies = keep;
    }

    pub fn set_keep_items(&mut self, keep: bool) {
        self.rules.keep_items = keep;
    }

    pub fn set_stock_sell_rate(&mut self, rate: f32) {
        self.rules.stock_sell_rate = rate;
    }

    pub fn set_count_humanless_members(&mut self, on: bool) {
        self.rules.count_humanless_members = on;
    }

    pub fn set_door_knocks(&mut self, on: bool) {
        self.rules.door_knocks = on;
    }

    pub fn request_reset(&mut self) {
        self.admin.reset_requested = true;
    }

    pub fn reset_lobby(&mut self) {
        self.reset_lobby_players_and_world();
    }

    pub fn intersections(&self) -> &[crate::world::streets::StreetIntersection] {
        &self.world.map.streets.intersections
    }

    pub fn clear_bullets(&mut self) {
        self.bullets.clear();
    }

    pub fn play_sound(&mut self, sound: Sound, pos: Vec3, volume: f32, pitch: f32) {
        let e = EventSound { sound_type: sound, pos, volume, pitch };

        self.events.push(Event { tick_created: self.tick, kind: ServerEvent::Sound(e) });
    }

    pub fn ground_below(&self, pos: Vec3) -> Option<f32> {
        let map = &self.world.map;
        line_intersect_level(&map.ground, &map.level.area, &map.level.meshes, pos + Vec3::Y * 8.0, pos - Vec3::Y * 256.0).map(|g| g.hit.pos.y)
    }
}
