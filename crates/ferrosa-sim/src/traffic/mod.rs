use glam::Vec3;
use ferrosa_physics::RotMatrix;

use crate::world::streets::StreetMap;

pub mod ai;
pub mod grid;
pub mod motion;
pub mod route;
pub mod spawn;

/// The route steps a car's record holds.
pub const MAX_ROUTE: usize = 63;
/// The streets the Round City traffic hack keeps cars off and treats as one loop.
pub const LOOP_STREET: i32 = 67;
pub const LOOP_STREET_2: i32 = 68;
pub const HIDDEN_STREET: i32 = 23;
/// Each street lists at most this many of the cars on it.
pub const MAX_STREET_CARS: usize = 256;

/// is_bot: driven by the traffic, parked, or taken over by a player.
pub const PARKED: i32 = 0;
pub const DRIVEN: i32 = 1;
pub const TAKEN: i32 = 2;

/// One step of a planned route: the street, the end it is driven from (0 from its first intersection) and the turn onto
/// it (-1 left, 1 right).
#[derive(Clone, Copy, Debug, Default)]
pub struct RouteStep {
    pub street: i32,
    pub end: i32,
    pub turn: i32,
}

/// A traffic car: a car that drives itself along the streets, simulated on its own away from
/// players and as a real vehicle near them.
#[derive(Clone, Debug)]
pub struct TrafficCar {
    pub kind: ferrosa_protocol::clientbound::game::VehicleKind,
    /// The bot driving it and the vehicle it is while near a player, -1 for none.
    pub human: i32,
    pub vehicle: i32,
    /// How near the nearest player is: 2 within range, 1 just outside it, 0 away.
    pub state: i32,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub rot: RotMatrix,
    pub steer: f32,
    pub target_speed: f32,
    /// How far the body reaches ahead of and behind its position, and the distance between its axles.
    pub front: f32,
    pub rear: f32,
    pub wheelbase: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub is_bot: i32,
    pub is_aggressive: i32,
    pub route_index: i32,
    pub route_len: i32,
    pub route: [RouteStep; MAX_ROUTE],
    /// The street being driven (-1 while crossing an intersection), the end it was entered from and
    /// the intersection being crossed.
    pub street: i32,
    pub intersection: i32,
    /// The lane driven, the lane being moved to and the lane to take on the next street.
    pub lane: i32,
    pub next_lane: i32,
    pub target_lane: i32,
    /// How far the move to `next_lane` has got, 0 to 1.
    pub lane_change: f32,
    /// Ticks spent stopped while meaning to move; past 179 the car backs out.
    pub stuck: i32,
    /// The street and intersection the car's position is in.
    pub physical_street: i32,
    pub physical_intersection: i32,
    /// The car's front and back along its street from the street's first intersection.
    pub front_proj: f32,
    pub rear_proj: f32,
    /// How far the car reaches from each end of its street, farthest and nearest.
    pub far: [f32; 2],
    pub near: [f32; 2],
    /// The lanes of its street the car covers.
    pub lane_mask: i32,
    pub color: i32,
}

impl TrafficCar {
    pub(crate) fn step(&self, i: i32) -> RouteStep {
        usize::try_from(i).ok().and_then(|i| self.route.get(i)).copied().unwrap_or_default()
    }

    pub(crate) fn speed(&self) -> f32 {
        self.vel.length()
    }
}

/// An intersection's traffic lights: straight on and right for
/// each way in (0 to 3) and left (4 to 7), 2 green, 1 amber, 0 red.
#[derive(Clone, Copy, Debug, Default)]
pub struct Signals {
    pub phase: i32,
    pub timer: i32,
    pub cycle: i32,
    pub lights: [i32; 8],
}

/// The traffic: its cars, the cars on each street and the intersections' lights.
#[derive(Clone, Debug, Default)]
pub struct Traffic {
    pub cars: Vec<TrafficCar>,
    /// How many cars have been made in each slot.
    pub generations: Vec<i32>,
    pub street_cars: Vec<Vec<usize>>,
    pub signals: Vec<Signals>,
    /// hack_roundcity_traffic: on Round City, cars keep off two streets and treat the loop as one.
    pub roundcity: bool,
    /// The co-op wave, which speeds up aggressive cars.
    pub coop_level: i32,
    pub coop: bool,
}

impl Traffic {
    pub(crate) fn new(map: &StreetMap, roundcity: bool) -> Self {
        Traffic {
            street_cars: vec![Vec::new(); map.streets.len()],
            signals: vec![Signals::default(); map.intersections.len()],
            roundcity,
            ..Default::default()
        }
    }
}

pub(crate) fn maxss(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

pub(crate) fn minss(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// get_random_street: a street with at most three lanes each way, rerolled up to 1024 times.
pub fn random_street(map: &StreetMap) -> i32 {
    let n = map.streets.len() as i32;
    let wide = |s: i32| map.streets.get(s as usize).is_some_and(|st| st.left_lanes > 3 || st.right_lanes > 3);
    let mut r = rand::random_range(0..n);
    for _ in 0..1024 {
        if !wide(r) {
            break;
        }
        r = rand::random_range(0..n);
    }
    r
}

pub(crate) fn lane_count(map: &StreetMap, s: i32) -> i32 {
    map.streets.get(s as usize).map_or(0, |st| st.lanes.len() as i32)
}

pub(crate) fn left_lanes(map: &StreetMap, s: i32) -> i32 {
    map.streets.get(s as usize).map_or(0, |st| st.left_lanes)
}
