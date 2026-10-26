//! The lab computers a mission's document can sit on, and the base computers that read them.

use ferrosa_wit::{BuildingKind, server};

use super::MissionSpot;
use crate::World;

/// A computer's volume holds 10000, a disk 360.
const COMPUTER_CAPACITY: i32 = 10000;

impl World {
    /// init_mission_spots: each base computer gets a volume with MISSION.EXE and DECRYPT.EXE for its corporation,
    /// and each lab becomes a mission spot with a volume per computer.
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
                        let Some(volume) = server::new_volume(COMPUTER_CAPACITY) else { continue };

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
                        let Some(volume) = server::new_volume(COMPUTER_CAPACITY) else { continue };

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
