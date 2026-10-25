use ferrosa_physics::rotation::{IDENTITY, rotate_orientation};
use ferrosa_protocol::clientbound::game::VehicleKind;
use glam::Vec3;

use super::{RouteStep, Traffic, TrafficCar, route::plan_route};
use crate::{vehicle::types::VehicleType, world::map::Map};

fn car_kind(roll: u32) -> VehicleKind {
    match roll {
        0..=2 => VehicleKind::Van,
        3..=8 => VehicleKind::TownCar,
        9..=12 => VehicleKind::Minivan,
        13 => VehicleKind::Van,
        _ => VehicleKind::Hatchback,
    }
}

/// create_traffic3: `count` cars on random streets, ends and lanes, each partway along its lane; with `parked_cars`
/// (eliminator) some are parked in the third lane of flat three lane streets.
pub fn create_traffic(traffic: &mut Traffic, map: &Map, types: &[VehicleType], parked_cars: bool, count: u32) {
    let streets = &map.streets;
    let n = streets.streets.len() as i32;

    for _ in 0..count {
        let slot = rand::random_range(0..2);
        let mut street = rand::random_range(0..n);

        if traffic.roundcity {
            while street == 23 || street == 67 {
                street = rand::random_range(0..n);
            }
        }

        let mut lane = rand::random_range(0..2);
        let st = &streets.streets[street as usize];

        let flat_three =
            parked_cars && streets.intersections[st.intersections[0]].pos.y == streets.intersections[st.intersections[1]].pos.y && st.left_lanes == 3;
        if flat_three {
            lane = rand::random_range(0..3);
        }

        let kind = car_kind(rand::random_range(0..16));
        let byte = rand::random_range(0..256);
        let color = rand::random_range(0..6);
        let id = create_traffic_car(traffic, map, types, kind, color, street, slot, lane, byte as f32 * 1.0 / 256.0);

        if lane == 2 {
            traffic.cars[id].is_bot = 0;
        }
    }
}

/// create_traffic_car: a car on a street's lane `lane` from the middle (towards its first intersection's side for
/// slot 0), `progress` of the way along, on the ground, with a route to a random street.
#[allow(clippy::too_many_arguments)]
pub fn create_traffic_car(
    traffic: &mut Traffic,
    map: &Map,
    types: &[VehicleType],
    kind: VehicleKind,
    color: i32,
    street: i32,
    slot: i32,
    lane: i32,
    progress: f32,
) -> usize {
    let streets = &map.streets;
    let st = &streets.streets[street as usize];
    let n = st.lanes.len() as i32;
    let mut k = (n >> 1) - lane - 1;

    if k < 0 {
        k = 0;
    }

    if n <= k {
        k = n - 1;
    }

    let mut dir = st.dir;

    let (from, dx, dz) = if slot == 0 {
        let l = st.lanes[(n - k - 1) as usize];
        (l.start, l.end.x - l.start.x, l.end.z - l.start.z)
    } else {
        dir = -dir;
        let l = st.lanes[k as usize];
        (l.end, l.start.x - l.end.x, l.start.z - l.end.z)
    };

    let len = (dz * dz + (dx * dx + 0.0)).sqrt();
    let p = progress * len;
    let mut yaw = if st.north_south != 0 { -90.0_f32.to_radians() } else { -180.0_f32.to_radians() };

    if slot == 0 {
        yaw = (yaw as f64 + 180.0_f64.to_radians()) as f32;
    }

    let pos = dir * p + from;
    let ends = st.intersections.map(|i| streets.intersections[i].world_pos.y);
    let top = ends[1].max(ends[0].max(pos.y)) + 2.0;
    let up = Vec3::Y;
    let start = Vec3::new(pos.x, top, pos.z);
    let end = Vec3::new(pos.x + up.x * -128.0, top + up.y * -128.0, -128.0 * up.z + pos.z);
    let ground = crate::world::trace::line_intersect_level(&map.ground, &map.level.area, &map.level.meshes, start, end).map_or(top, |h| h.hit.pos.y);
    let wheel_y = types.get(kind as usize).and_then(|t| t.wheels.first()).map_or(0.0, |w| w.local_pos.y);
    let left = st.left_lanes;
    let car_lane = if slot != 0 { left - 1 - lane } else { lane + left };
    let id = traffic.cars.len();

    let mut car = TrafficCar {
        kind,
        human: -1,
        vehicle: -1,
        state: 0,
        pos: Vec3::new(pos.x, (0.375 - wheel_y) + ground, pos.z),
        vel: Vec3::ZERO,
        yaw,
        rot: IDENTITY,
        steer: 0.0,
        target_speed: 0.0,
        front: 3.5,
        rear: -0.5,
        wheelbase: 3.0,
        bounds_min: Vec3::ZERO,
        bounds_max: Vec3::ZERO,
        is_bot: 1,
        is_aggressive: 0,
        route_index: 0,
        route_len: 0,
        route: [RouteStep::default(); 63],
        street,
        intersection: -1,
        lane: car_lane,
        next_lane: car_lane,
        target_lane: 0,
        lane_change: 0.0,
        stuck: 0,
        physical_street: 0,
        physical_intersection: 0,
        front_proj: 0.0,
        rear_proj: 0.0,
        far: [0.0; 2],
        near: [0.0; 2],
        lane_mask: 0,
        color,
    };

    let to = rand::random_range(0..streets.streets.len() as i32);
    let to_slot = rand::random_range(0..2);
    plan_route(&mut car, streets, street, slot, to, to_slot);

    let mut rot = IDENTITY;
    let axis = rot[1];
    rotate_orientation(&mut rot, axis, (yaw as f64 + 90.0_f64.to_radians()) as f32);

    car.rot = rot;
    traffic.cars.push(car);

    if traffic.generations.len() <= id {
        traffic.generations.resize(id + 1, 0);
    }

    traffic.generations[id] += 1;

    id
}
