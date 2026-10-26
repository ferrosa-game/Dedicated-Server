//! One way the intel reaches the city: a blue disk left in a lab's lobby, with a memo saying which lab.

use ferrosa_wit::{ItemKind, Mat3, server};

use super::DISK_CAPACITY;
use crate::{World, state::CORPORATIONS};

impl World {
    /// The intel left on a blue disk in a random lab's lobby, and the memo saying where.
    pub(super) fn intel_disk_memo(&mut self, t: usize, file: &str, content: i32) {
        let spots = self.state.missions.spots.len();

        if spots == 0 {
            return;
        }

        let building = self.state.missions.spots[rand::random_range(0..spots)].building;
        let Some(b) = server::get_building(building) else { return };
        let (lobby, (a, b)) = (b.lobby(), b.streets());

        if let Some(disk) = server::spawn_item_by_id(ItemKind::DiskBlue as u32, lobby, Mat3::IDENTITY)
            && let Some(volume) = server::new_volume(DISK_CAPACITY)
        {
            disk.set_volume(&volume);
            volume.add_file_of_content(file, content);
        }

        let memo = format!(
            "We have intercepted communications\nThe disk with the info in the lobby of the lab at\n{} and {}\nFilename: {file}",
            server::street_name(a),
            server::street_name(b)
        );

        server::place_team_memo(CORPORATIONS[t], &memo);
    }
}
