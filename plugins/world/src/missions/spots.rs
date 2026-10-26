use ferrosa_wit::{BuildingKind, server};

use super::MissionSpot;
use crate::World;

impl World {
    pub fn init_mission_spots(&mut self) {
        self.state.missions.spots.clear();

        for c in &mut self.state.corps {
            c.computers.clear();
        }

        for b in server::buildings() {
            match b.kind() {
                BuildingKind::CorporationBase => {
                    let team = b.team();

                    for (m, _) in b.computers().iter().enumerate() {
                        let Some(volume) = server::new_volume(10_000) else { continue };

                        b.set_computer_volume(m as u32, &volume);
                        volume.add_file_of_content("MISSION.EXE", 0);
                        volume.add_file_of_content("DECRYPT.EXE", 0);

                        if let Some(k) = team.and_then(crate::state::World::team_index) {
                            self.state.corps[k].computers.push(volume.id() as i32);
                        }
                    }
                }

                BuildingKind::Lab => {
                    let mut volumes = Vec::new();

                    for (m, _) in b.computers().iter().enumerate() {
                        let Some(volume) = server::new_volume(10_000) else { continue };

                        b.set_computer_volume(m as u32, &volume);
                        volumes.push(volume.id() as i32);
                    }

                    self.state.missions.spots.push(MissionSpot { building: b.id(), used: 0, volumes });
                }

                _ => {}
            }
        }
    }
}
