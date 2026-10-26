use ferrosa_wit::{ItemKind, Mat3, server};

use crate::{World, state::CORPORATIONS};

impl World {
    pub(super) fn intel_disk_memo(&mut self, t: usize, file: &str, content: i32) {
        let spots = self.state.missions.spots.len();

        if spots == 0 {
            return;
        }

        let building = self.state.missions.spots[rand::random_range(0..spots)].building;
        let Some(b) = server::get_building(building) else { return };
        let (lobby, (a, b)) = (b.lobby(), b.streets());

        if let Some(disk) = server::spawn_item_by_id(ItemKind::DiskBlue as u32, lobby, Mat3::IDENTITY)
            && let Some(volume) = server::new_volume(360)
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
