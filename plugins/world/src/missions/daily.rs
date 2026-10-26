//! The day's missions: seven projects spread across the labs, with decoy documents among them.

use ferrosa_wit::server;

use super::{DAILY_MISSIONS, INTEL_CONTENT, MISSIONS, Mission, names::{NAMES, NAME_FLAGS, PICK_TRIES, PROJECTS, pick_name, pick_volume}, text::mission_string};
use crate::World;

const DECOYS: i32 = 64;

const SHUFFLES: i32 = 64;

/// A mission's reward is 5000 to 15000 in steps of 2500, five times that for the day's boosted mission.
const REWARD_BASE: i32 = 5000;

const REWARD_STEP: i32 = 2500;

const BOOST: i32 = 5;

impl World {
    /// The day's world missions: the names freed, one of the 7 boosted, the missions made and their documents spread
    /// out.
    pub fn start_world_missions(&mut self) {
        self.state.missions.names_used = vec![false; NAME_FLAGS];
        self.state.missions.boosted = rand::random_range(0..DAILY_MISSIONS) as i32;

        for _ in 0..DAILY_MISSIONS {
            self.create_world_mission();
        }

        self.distribute_documents();
    }

    /// create_world_mode_mission: a project at the least used lab: its document on one of the lab's computers, a
    /// reward for three corporations (raised by their manager's corporate rating) and its INTEL file.
    fn create_world_mission(&mut self) {
        if self.state.missions.missions.len() < MISSIONS {
            self.state.missions.missions.resize(MISSIONS, Mission::default());
        }

        let Some(idx) = self.state.missions.missions.iter().position(|m| !m.active) else { return };
        self.state.missions.missions[idx] = Mission { active: true, ..Default::default() };
        let n = self.state.missions.spots.len();

        if n == 0 {
            return;
        }

        let least = self.state.missions.spots.iter().map(|s| s.used).fold(65536, i32::min);
        let mut r = rand::random_range(0..n);
        let mut tries = PICK_TRIES;

        while self.state.missions.spots[r].used > least {
            r = rand::random_range(0..n);
            tries -= 1;

            if tries == 0 {
                break;
            }
        }

        self.state.missions.spots[r].used += 1;
        let building = self.state.missions.spots[r].building;
        let Some(content) = server::alloc_content() else { return };
        self.state.missions.missions[idx].document = content;
        let vol = pick_volume(&self.state.missions.spots[r].volumes);
        let k = pick_name(&mut self.state.missions.names_used, true);
        let doc = NAMES[PROJECTS + k];

        if let Some(volume) = server::get_volume(vol) {
            volume.add_file_of_content(&format!("{doc}.DOC"), content);
        }

        let high_value = rand::random_ratio(1, 5);
        let mut reward = rand::random_range(0..5) * REWARD_STEP + REWARD_BASE;

        if self.state.missions.boosted == idx as i32 {
            reward *= BOOST;
        }

        self.state.missions.missions[idx].reward = reward;

        for _ in 0..3 {
            let t = rand::random_range(0..6);
            self.state.missions.missions[idx].teams[t].value = self.manager_bonus(t) * reward / 100 + reward;
        }

        let (a, b) = server::get_building(building).map_or((-1, -1), |b| b.streets());
        let (a, b) = (server::street_name(a), server::street_name(b));
        let intel = idx as i32 + INTEL_CONTENT;
        let text = mission_string(doc, &a, &b, NAMES.get(idx).copied().unwrap_or(""), high_value);

        server::set_content_text(intel, &text);
        server::encipher_content(intel);

        let m = &mut self.state.missions.missions[idx];
        m.intel = intel;
        m.filename = format!("INTEL{idx:02}.CDE");
        m.names = [doc.to_string(), a, b, doc.to_string()];
        m.high_value = high_value;

        for t in &mut m.teams {
            t.word = "Open".into();
        }
    }

    /// world_distribute_document_missions: 64 decoy documents across each lab's computers, then each computer's
    /// files shuffled.
    fn distribute_documents(&mut self) {
        for s in 0..self.state.missions.spots.len() {
            for _ in 0..DECOYS {
                let vol = pick_volume(&self.state.missions.spots[s].volumes);
                let k = pick_name(&mut self.state.missions.names_used, false);
                let name = format!("{}.DOC", NAMES[PROJECTS + k]);
                let Some(volume) = server::get_volume(vol) else { continue };

                if volume.files().iter().any(|f| f.name() == name) {
                    continue;
                }

                if let Some(content) = server::alloc_content() {
                    volume.add_file_of_content(&name, content);
                }
            }
        }

        let spots: Vec<Vec<i32>> = self.state.missions.spots.iter().map(|s| s.volumes.clone()).collect();

        for volumes in spots {
            for v in volumes {
                let Some(volume) = server::get_volume(v) else { continue };
                let n = volume.files().len();

                if n <= 1 {
                    continue;
                }

                for _ in 0..SHUFFLES {
                    let a = rand::random_range(0..n);
                    let b = rand::random_range(0..n);

                    if a != b {
                        volume.swap_files(a as u32, b as u32);
                    }
                }
            }
        }
    }
}
