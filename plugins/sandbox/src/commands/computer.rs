use ferrosa_wit::{Player, Vec3, server};

const TABLE_CENTRE: f32 = 0.75;
const TABLE_TOP: f32 = 0.5;
const COMPUTER_CAPACITY: i32 = 10000;

pub fn spawn(p: &Player) {
    let facing = p.facing();
    let spot = p.human().map_or(p.camera(), |h| h.pos()) - facing.z * 2.0;
    let floor = server::ground_below(spot).unwrap_or(spot.y);
    let base = Vec3::new(spot.x, floor, spot.z);

    if server::spawn_item("table", base + Vec3::new(0.0, TABLE_CENTRE, 0.0), facing).is_none() {
        p.message("No room for a table");
        return;
    }

    let Some(item) = server::spawn_item("computer", base + Vec3::new(0.0, TABLE_CENTRE + TABLE_TOP + 0.125, 0.0), facing) else {
        p.message("No room for a computer");
        return;
    };

    let Some(volume) = server::new_volume(COMPUTER_CAPACITY) else {
        p.message("Computer spawned without a disk");
        return;
    };

    item.set_volume(&volume);

    volume.add_file("DECRYPT.EXE", "");

    p.message("Computer spawned. Pick it up to type on it.");
}
