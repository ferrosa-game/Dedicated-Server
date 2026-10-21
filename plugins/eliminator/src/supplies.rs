use ferrosa_wit::{Base, Mat3, Team, Vec3, server};

use crate::TEAMS;

const TABLE_STEP: f32 = 0.375;
const GUNS: [(&str, &str); 5] = [("pistol", "pistolmag"), ("mp5", "mp5mag"), ("ak47", "ak47mag"), ("m16", "m16mag"), ("uzi", "uzimag")];

const MISSION_SPOTS: [Vec3; 8] = [
    // Hondo Park
    Vec3::new(1718.0, 46.0, 1262.0),
    // Rio Granary
    Vec3::new(1778.0, 30.0, 1006.0),
    // Isle of Burgers
    Vec3::new(1470.0, 50.0, 1494.0),
    // Park above the Kamel Building
    Vec3::new(1282.0, 62.0, 1534.0),
    // Bobson Dugnutt Museum
    Vec3::new(1674.0, 66.0, 1510.0),
    // The Mall
    Vec3::new(1346.0, 38.0, 1298.0),
    // Gas Station
    Vec3::new(1606.0, 26.0, 1166.0),
    // Red Cube Park
    Vec3::new(1546.0, 38.0, 1278.0),
];

fn step(p: Vec3, along: Vec3) -> Vec3 {
    along * TABLE_STEP + p
}

fn frame(base: &Base) -> Mat3 {
    let (s, c) = (base.table_orientation + std::f32::consts::PI).sin_cos();

    Mat3 { x: Vec3::new(c, 0.0, s), y: Vec3::new(0.0, 1.0, 0.0), z: Vec3::new(-s, 0.0, c) }
}

fn random_gun() -> (&'static str, &'static str) {
    GUNS[rand::random_range(0..GUNS.len())]
}

fn spread(by: f32) -> Vec3 {
    Vec3::new(rand::random_range(-by..=by), 0.0, rand::random_range(-by..=by))
}

pub(crate) fn stock_tables() {
    for (k, team) in TEAMS.into_iter().enumerate() {
        let Some(corp) = server::get_corporation(team) else { continue };
        let car = match rand::random_range(0..16) {
            0..=5 => "hatchback",
            6..=10 => "beamer",
            _ => "test",
        };

        if let Some(kind) = server::find_vehicle_kind(car)
            && let Some(vehicle) = corp.spawn_vehicle(kind, rand::random_range(0..6))
        {
            vehicle.park_in_traffic();
        }

        let Some(base) = corp.base() else { continue };
        let rot = frame(&base);
        let mut p = base.table;

        for _ in 0..3 {
            server::spawn_item("pistol", p, rot);

            for _ in 0..3 {
                p = step(p, rot.x);
                server::spawn_item("pistolmag", p, rot);
            }

            p = step(p, rot.x);
        }

        let two = rand::random::<bool>();
        server::spawn_item("bandage", p, rot);
        p = step(p, rot.x);

        if two {
            server::spawn_item("bandage", p, rot);
            p = step(p, rot.x);
        }

        if let Some(phone) = server::spawn_item("phone", p, rot) {
            phone.set_phone_number((k as i32 + 1) * 1111);
            phone.set_phone_texture(0);
        }

        p = step(p, rot.x);
        server::spawn_item("soccerball", p, rot);
    }
}

pub(crate) fn drop(ready: usize) {
    let spot = MISSION_SPOTS[rand::random_range(0..MISSION_SPOTS.len())];
    server::spawn_item("grenade", spot + spread(0.25), Mat3::IDENTITY);

    let spot = MISSION_SPOTS[rand::random_range(0..MISSION_SPOTS.len())];
    let (gun, mag) = random_gun();

    for name in std::iter::once(gun).chain(std::iter::repeat_n(mag, 3)) {
        server::spawn_item(name, spot + spread(1.0), Mat3::IDENTITY);
    }

    let mut tables = [ready > 14; 3];
    let first = rand::random_range(0..3);
    tables[first] = true;

    if ready > 7 {
        tables[(first + 1) % 3] = true;
    }

    for (team, restock_it) in TEAMS.into_iter().zip(tables) {
        if restock_it {
            restock(team);
        }
    }
}

fn restock(team: Team) {
    let Some(base) = server::get_corporation(team).and_then(|c| c.base()) else { return };
    let rot = frame(&base);
    let mut p = step(base.table, rot.x);
    let below = Vec3::new(p.x, p.y - 0.5, p.z);
    let cleared: Vec<Vec3> = (0..16).map(|k| rot.x * (k as f32 * TABLE_STEP) + below).collect();

    for item in server::items().into_iter().filter(|i| i.holder().is_none()) {
        let pos = item.pos();

        if cleared.iter().any(|&q| (pos - q).length() < 0.5) {
            item.delete();
        }
    }

    for _ in 0..3 {
        let (gun, mag) = random_gun();
        server::spawn_item(gun, p, rot);

        for _ in 0..3 {
            p = step(p, rot.x);
            server::spawn_item(mag, p, rot);
        }

        p = step(p, rot.x);
    }

    server::spawn_item("grenade", p, rot);
    p = step(p, rot.x);

    let two = rand::random::<bool>();
    server::spawn_item("bandage", p, rot);
    p = step(p, rot.x);

    if two {
        server::spawn_item("bandage", p, rot);
    }
}
