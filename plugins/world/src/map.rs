use ferrosa_wit::{BuildingKind, Mat3, VehicleKind, Vec3, server};

/// The trains stand this far above the station they start from.
const TRAIN_LIFT: f32 = 2.0;
const TRAIN_COLOUR: i32 = 1;

/// Three to six guards a lab gets, never more than it has posts for.
const GUARDS_MIN: i32 = 3;
const SPOT_TRIES: i32 = 512;

/// The NPC kind a lab guard is.
const LAB_GUARD: i32 = 0;

/// world_create_train: a train at each of the track's stations, facing the way the station does.
pub fn create_trains() {
    for (i, (p, angle)) in server::track_spawns().into_iter().enumerate() {
        let pos = Vec3::new(p.x, p.y + TRAIN_LIFT, p.z);
        let rot = Mat3::from_yaw(angle);

        if let Some(v) = server::spawn_vehicle_by_id(VehicleKind::Train as u32, TRAIN_COLOUR, pos, rot) {
            v.set_train_index(i as i32);
        }
    }
}

/// spawn_building_patrol_npcs: three to six guards (no more than the posts), each at a post not yet taken (up to 512
/// tries), facing the building's way, with its patrol route.
pub fn spawn_patrol_guards(building: u32) {
    let Some(b) = server::get_building(building) else { return };
    let spots = b.patrol();
    let n = spots.len();

    if n == 0 {
        return;
    }

    let a = rand::random_range(0..4);
    let b = rand::random_range(0..2);
    let base = a + b + GUARDS_MIN;
    let count = (rand::random_range(0..2) + base).min(n as i32);

    if count <= 0 {
        return;
    }

    let mut taken = vec![false; n];

    for _ in 0..count {
        let mut pick = rand::random_range(0..n);
        let mut tries = SPOT_TRIES;

        while taken[pick] {
            pick = rand::random_range(0..n);
            tries -= 1;

            if tries == 0 {
                break;
            }
        }

        taken[pick] = true;
        let spot = &spots[pick];

        if let Some(npc) = server::create_npc(LAB_GUARD, spot.pos, spot.facing) {
            server::set_npc_waypoints(npc, &spot.waypoints);
        }
    }
}

/// Every lab on the map gets its guards.
pub fn guard_the_labs() {
    for b in server::buildings().iter().filter(|b| b.kind() == BuildingKind::Lab) {
        spawn_patrol_guards(b.id());
    }
}
