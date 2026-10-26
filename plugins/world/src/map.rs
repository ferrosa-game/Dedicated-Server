use ferrosa_wit::{BuildingKind, Mat3, VehicleKind, Vec3, server};

pub fn create_trains() {
    for (i, (p, angle)) in server::track_spawns().into_iter().enumerate() {
        let pos = Vec3::new(p.x, p.y + 2.0, p.z);
        let rot = Mat3::from_yaw(angle);

        if let Some(v) = server::spawn_vehicle_by_id(VehicleKind::Train as u32, 1, pos, rot) {
            v.set_train_index(i as i32);
        }
    }
}

pub fn spawn_patrol_guards(building: u32) {
    let Some(b) = server::get_building(building) else { return };
    let spots = b.patrol();
    let n = spots.len();

    if n == 0 {
        return;
    }

    let a = rand::random_range(0..4);
    let b = rand::random_range(0..2);
    let base = a + b + 3;
    let count = (rand::random_range(0..2) + base).min(n as i32);

    if count <= 0 {
        return;
    }

    let mut taken = vec![false; n];

    for _ in 0..count {
        let mut pick = rand::random_range(0..n);
        let mut tries = 512;

        while taken[pick] {
            pick = rand::random_range(0..n);
            tries -= 1;

            if tries == 0 {
                break;
            }
        }

        taken[pick] = true;
        let spot = &spots[pick];

        if let Some(npc) = server::create_npc(0, spot.pos, spot.facing) {
            server::set_npc_waypoints(npc, &spot.waypoints);
        }
    }
}

pub fn guard_the_labs() {
    for b in server::buildings().iter().filter(|b| b.kind() == BuildingKind::Lab) {
        spawn_patrol_guards(b.id());
    }
}
