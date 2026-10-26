use ferrosa_wit::server;

use super::{Mission, names::{NAMES, pick_name, pick_volume}, text::mission_string};
use crate::World;

impl World {
    pub fn start_world_missions(&mut self) {
        self.state.missions.names_used = vec![false; 512];
        self.state.missions.boosted = rand::random_range(0..7) as i32;

        for _ in 0..7 {
            self.create_world_mission();
        }

        self.distribute_documents();
    }

    fn create_world_mission(&mut self) {
        if self.state.missions.missions.len() < 256 {
            self.state.missions.missions.resize(256, Mission::default());
        }

        let Some(idx) = self.state.missions.missions.iter().position(|m| !m.active) else { return };
        self.state.missions.missions[idx] = Mission { active: true, ..Default::default() };
        let n = self.state.missions.spots.len();

        if n == 0 {
            return;
        }

        let least = self.state.missions.spots.iter().map(|s| s.used).fold(65536, i32::min);
        let mut r = rand::random_range(0..n);
        let mut tries = 1024;

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
        let doc = NAMES[8 + pick_name(&mut self.state.missions.names_used, true)];

        if let Some(volume) = server::get_volume(vol) {
            volume.add_file_of_content(&format!("{doc}.DOC"), content);
        }

        let high_value = rand::random_ratio(1, 5);
        let mut reward = rand::random_range(0..5) * 2_500 + 5_000;

        if self.state.missions.boosted == idx as i32 {
            reward *= 5;
        }

        self.state.missions.missions[idx].reward = reward;

        for _ in 0..3 {
            let t = rand::random_range(0..6);
            self.state.missions.missions[idx].teams[t].value = self.manager_bonus(t) * reward / 100 + reward;
        }

        let (a, b) = server::get_building(building).map_or((-1, -1), |b| b.streets());
        let (a, b) = (server::street_name(a), server::street_name(b));
        let intel = idx as i32 + 200;
        let text = mission_string(doc, &a, &b, NAMES.get(idx).copied().unwrap_or(""), high_value);

        server::set_content_text(intel, &text);
        server::encipher_content(intel);

        let mission = &mut self.state.missions.missions[idx];
        mission.intel = intel;
        mission.filename = format!("INTEL{idx:02}.CDE");
        mission.names = [doc.to_string(), a, b, doc.to_string()];
        mission.high_value = high_value;

        for team in &mut mission.teams {
            team.word = "Open".into();
        }
    }
    fn distribute_documents(&mut self) {
        for s in 0..self.state.missions.spots.len() {
            for _ in 0..64 {
                let vol = pick_volume(&self.state.missions.spots[s].volumes);
                let k = pick_name(&mut self.state.missions.names_used, false);
                let name = format!("{}.DOC", NAMES[8 + k]);
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

                for _ in 0..64 {
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
