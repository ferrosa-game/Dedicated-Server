//! The intel memo a corporation is faxed: which mission it is told about, and which way the file reaches the city.

use ferrosa_wit::server;

use super::{DAILY_MISSIONS, clock_time, intel_car::INTEL_TIME, text::{project_part, streets_part, worth_part}};
use crate::{World, state::CORPORATIONS};

/// A corporation's intel once its memo is faxed.
const INTEL_FAXED: i32 = 11;

/// Corporations give up looking for a mission worth something to one of them after 100 tries.
const MEMO_TRIES: i32 = 100;

/// A memo's three INTEL parts come in a shuffled order (64 swaps).
const PART_SHUFFLES: i32 = 64;

impl World {
    /// generate_team_mission_memo: the corporation is told about one of the day's missions it has asked least about
    /// (among those not yet traded) that some corporation with players wants. An encrypted INTEL file describes it,
    /// and the file is either left on a disk in a lab's lobby or carried by a town car, which the memo tells of.
    pub fn generate_team_mission_memo(&mut self, t: usize) {
        if self.state.missions.missions.len() < DAILY_MISSIONS {
            return;
        }

        let daily = &self.state.missions.missions[..DAILY_MISSIONS];
        let least_asked = daily.iter().map(|m| m.teams[t].status).fold(256, i32::min);
        let least_state = daily.iter().map(|m| m.state).fold(256, i32::min);

        if least_state > 0 {
            return;
        }

        let list: Vec<usize> = (0..DAILY_MISSIONS).filter(|&i| daily[i].teams[t].status == least_asked && daily[i].state == least_state).collect();

        if list.is_empty() {
            return;
        }

        let mut m = 0;

        for tries in 1.. {
            m = list[rand::random_range(0..list.len())];
            let wanted = (0..6).any(|j| self.corp_player_count(j) > 0 && self.state.missions.missions[m].teams[j].value > 0);

            if wanted || tries >= MEMO_TRIES {
                break;
            }
        }

        let mut order = [0, 1, 2];

        for _ in 0..PART_SHUFFLES {
            let a = rand::random_range(0..3);
            let b = rand::random_range(0..3);
            order.swap(a, b);
        }

        self.state.missions.missions[m].teams[t].status += 1;

        if let Some(corp) = server::get_corporation(CORPORATIONS[t]) {
            corp.set_intel(INTEL_FAXED);
        }

        let boosted = self.state.missions.boosted == m as i32;
        let names = self.state.missions.missions[m].names.clone();
        let mut text = String::new();

        for part in order {
            text += &match part {
                0 => streets_part(&names[1], &names[2]),
                1 => project_part(&names[3]),
                _ => worth_part(boosted).to_string(),
            };
        }

        let Some(content) = server::alloc_content() else { return };
        server::set_content_text(content, &text);
        server::encipher_content(content);

        self.state.missions.intel_files += 1;
        let file = format!("INTEL{:02}.CDE", self.state.missions.intel_files);

        if rand::random::<bool>() {
            return self.intel_disk_memo(t, &file, content);
        }

        let expiry = server::world_time() + INTEL_TIME;
        let Some(car) = self.spawn_mission_intel_car(&file, content, expiry) else { return };
        let (hour, minute, pm) = clock_time(expiry);
        let route = server::traffic_car_route(car);
        let (Some(&first), Some(&last)) = (route.first(), route.last()) else { return };
        let mut memo = String::from("There is in a town car driving around the city with\nvaluable information.\n");

        if let Some((a, b)) = server::street_corner(first) {
            memo += &format!("The car will be driving from {a} and {b}\n");
        }

        if let Some((a, b)) = server::street_corner(last) {
            memo += &format!("to {a} and {b}.\n");
        }

        memo += &format!("This mission expires at {hour}:{minute:02} {}.\n", if pm { "PM" } else { "AM" });
        server::place_team_memo(CORPORATIONS[t], &memo);
    }
}
