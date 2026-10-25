use glam::Vec3;

use super::{LOOP_STREET, LOOP_STREET_2, Traffic, grid::lane_clearance, lane_count, left_lanes, random_street, route::plan_route};
use crate::{
    time::{TICKS_PER_SECOND_F32, game_seconds}, world::streets::{Lane, MapStreet, StreetMap},
};

const BEZIER_EPSILON: f32 = 1.0 / 65536.0;

fn lane_at(st: &MapStreet, i: i32) -> Lane {
    usize::try_from(i).ok().and_then(|i| st.lanes.get(i)).copied().unwrap_or_default()
}

fn sincos(yaw: f32) -> (f32, f32) {
    let (s, c) = (yaw as f64).sin_cos();
    (s as f32, c as f32)
}

/// The steering angle that turns from `yaw` towards `target`, wrapped the short way and clamped.
fn steer_toward(yaw: f32, target: f32, limit: f64, max: f32) -> f32 {
    let mut a = target;

    if ((yaw - a) as f64) >= 180.0_f64.to_radians() {
        a = (a as f64 + 360.0_f64.to_radians()) as f32;
    }

    let mut diff = a - yaw;

    if (diff as f64) >= 180.0_f64.to_radians() {
        diff = a - (yaw as f64 + 360.0_f64.to_radians()) as f32;
    }

    let d = diff as f64;

    if -limit > d {
        -max
    } else if d > limit {
        max
    } else {
        diff
    }
}

/// The co-op speed for aggressive cars: (wave * `per` + 16) / 60.
fn coop_speed(traffic: &Traffic, per: i32) -> f32 {
    (traffic.coop_level * per + 16) as f32 / TICKS_PER_SECOND_F32
}

/// The stuck count: it climbs while the car is stopped but means to go forwards, and runs on to 319 once past 179.
fn update_stuck(traffic: &mut Traffic, id: usize) {
    let c = &mut traffic.cars[id];

    if c.stuck > game_seconds(3) {
        c.stuck += 1;

        if c.stuck > game_seconds(5) {
            c.stuck = 0;
        }

        return;
    }

    let v = c.vel;
    let speed = v.length();

    if 1.0 / 60.0 > speed && (c.rot[2].dot(v)) + c.target_speed > 1.0 / 15.0 {
        c.stuck += 1;
    } else {
        c.stuck = 0;
    }
}

/// Replans from `street` once the car reaches the end of its route; an aggressive car heads for a random street.
fn replan(traffic: &mut Traffic, map: &StreetMap, id: usize, street: i32, mut dest: i32) {
    let n = map.streets.len() as i32;

    if traffic.cars[id].is_aggressive != 0 {
        dest = 0;

        if n != 0 {
            dest = random_street(map);
        }
    }

    let r = rand::random_range(0..2);
    let c = &mut traffic.cars[id];
    let end = c.step(c.route_index).end;
    plan_route(c, map, street, end, dest, r);
}

/// The Round City loop: a car on one loop street heading the wrong way is turned onto the other end and replanned.
fn roundcity_flip(traffic: &mut Traffic, map: &StreetMap, id: usize, street: i32) {
    let (other, end) = if street == LOOP_STREET { (LOOP_STREET_2, 0) } else { (LOOP_STREET, 1) };
    let n = map.streets.len() as i32;
    let c = &mut traffic.cars[id];
    c.lane = lane_count(map, street) - c.lane - 1;
    let ri = c.route_index as usize;

    if let Some(step) = c.route.get_mut(ri) {
        step.end = end;
    }

    let mut dest = other;

    if !rand::random::<bool>() {
        dest = rand::random_range(0..n);
    }

    if traffic.cars[id].is_aggressive != 0 {
        dest = n;

        if n != 0 {
            dest = random_street(map);
        }
    }

    let r = rand::random_range(0..2);
    let c = &mut traffic.cars[id];
    let e = c.step(c.route_index).end;
    plan_route(c, map, street, e, dest, r);
}

/// The lane a car moves to on its street, kept to the lanes heading its way (not the outermost).
fn clamp_lane(map: &StreetMap, street: i32, reverse: bool, mut nl: i32) -> i32 {
    let left = left_lanes(map, street);

    if reverse {
        if nl <= 0 {
            nl = 1;
        }

        if !(left > nl) {
            nl = left - 1;
        }
    } else {
        if left > nl {
            nl = left;
        }

        let n = lane_count(map, street);

        if n - 1 <= nl {
            nl = n - 2;
        }
    }

    nl
}

/// Now and then a car picks a lane for its next turn, or (rarely) wanders a lane over.
fn pick_lane(traffic: &mut Traffic, map: &StreetMap, id: usize, street: i32) -> i32 {
    let c = &mut traffic.cars[id];
    let mut nl = c.next_lane;

    if rand::random_ratio(127, 128) {
        return nl;
    }

    let turn = c.step(c.route_index + 1).turn;

    if turn == 0 && rand::random_ratio(1, 256) {
        nl += if rand::random::<bool>() { 1 } else { -1 };
    }

    let reverse = c.step(c.route_index).end != 0;
    nl = match (turn, reverse) {
        (-1, true) | (1, false) => nl + 1,
        (-1, false) | (1, true) => nl - 1,
        _ => nl,
    };
    nl = clamp_lane(map, street, reverse, nl);
    c.next_lane = nl;
    nl
}

/// ai_traffic_car: lane keeping along the street, the lane for the next turn, slowing for lights and the cars ahead,
/// and the curve across an intersection.
pub fn ai_traffic_car(traffic: &mut Traffic, map: &StreetMap, id: usize) {
    update_stuck(traffic, id);

    let street = traffic.cars[id].street;

    if street == -1 {
        return cross_intersection(traffic, map, id);
    }

    keep_route(traffic, map, id, street);

    let st = &map.streets[street as usize];
    let lane = traffic.cars[id].lane;

    let to = steer_along_lane(traffic, map, id, street);
    pick_target_lane(traffic, map, id);

    let (remaining, junction, way) = approach_junction(traffic, map, id, st, to);
    let approach = slow_for_signal(traffic, id, remaining, junction, way);
    let queued = slow_for_queue(traffic, id, remaining, approach);
    let Some(blocked) = slow_for_clearance(traffic, map, id, st, queued) else { return };
    let c = &mut traffic.cars[id];

    if blocked && 0.3 > c.target_speed && c.next_lane == lane {
        let nl = c.next_lane + if rand::random::<bool>() { 1 } else { -1 };
        c.next_lane = nl.max(0).min(st.lanes.len() as i32 - 1);
    }

    if 2.0 / 15.0 > c.target_speed {
        c.target_speed = 2.0 / 15.0;
    }
}

/// The Round City loop flip, and a new route once the car reaches the end of its current one.
fn keep_route(traffic: &mut Traffic, map: &StreetMap, id: usize, street: i32) {
    if traffic.roundcity && (street == LOOP_STREET || street == LOOP_STREET_2) {
        let c = &traffic.cars[id];
        let end = c.step(c.route_index).end;
        let wrong_way = if street == LOOP_STREET { end != 0 } else { end == 0 };

        if wrong_way {
            roundcity_flip(traffic, map, id, street);
        }
    }

    let c = &mut traffic.cars[id];

    if c.route_len - 1 <= c.route_index {
        c.street = street;
        c.intersection = -1;

        let dest = match traffic.roundcity {
            true if street == LOOP_STREET => LOOP_STREET_2,
            true => LOOP_STREET,
            false => rand::random_range(0..map.streets.len() as i32),
        };

        replan(traffic, map, id, street, dest);
    }
}

/// Eases into the next lane and steers for a point ahead on the lane. Returns the lane's far end.
fn steer_along_lane(traffic: &mut Traffic, map: &StreetMap, id: usize, street: i32) -> Vec3 {
    let st = &map.streets[street as usize];
    let c = &traffic.cars[id];
    let lane = c.lane;
    let reverse = c.step(c.route_index).end != 0;
    let l = lane_at(st, lane);
    let (mut from, to) = if reverse { (l.end, l.start) } else { (l.start, l.end) };
    let nl = if c.is_aggressive != 0 || lane != c.next_lane { c.next_lane } else { pick_lane(traffic, map, id, street) };
    let c = &mut traffic.cars[id];

    if lane != nl {
        let p = c.lane_change;
        let s = (p * p) * (3.0 - (p + p));
        let np = (c.vel.length() * 0.125).min(1.0 / 120.0) + p;
        c.lane_change = np;

        if np > 1.0 {
            c.lane_change = 0.0;
            c.lane = c.next_lane;
        }

        let t = lane_at(st, c.next_lane).start;
        from = from + (t - from) * s;
    }

    let d = if reverse { -st.dir } else { st.dir };
    let pos = c.pos;
    let off = (from - pos).dot(st.side).abs();
    let a = d * (10.0 - off * 0.25).max(0.0) + pos;
    let proj = (a - from).dot(d);
    let aim_z = proj * d.z + from.z - pos.z;
    let aim_x = from.x + d.x * proj - pos.x;
    let target_yaw = (aim_z as f64).atan2(aim_x as f64) as f32;
    let want = steer_toward(c.yaw, target_yaw, 45.0_f64.to_radians(), 45.0_f32.to_radians());

    c.steer += (want - c.steer).clamp(-(1.0 / 16.0), 1.0 / 16.0);

    to
}

/// The lane to arrive in on the next street: one of the two outer lanes on its side for a turn, else the same lane.
fn pick_target_lane(traffic: &mut Traffic, map: &StreetMap, id: usize) {
    let c = &mut traffic.cars[id];
    c.target_lane = c.lane;

    let r = rand::random_range(0..2);
    let nxt = c.step(c.route_index + 1);

    if nxt.turn == 1 {
        c.target_lane = if nxt.end != 0 { r + 1 } else { !r + lane_count(map, nxt.street) - 1 };
    } else if nxt.turn == -1 {
        let left = left_lanes(map, nxt.street);

        c.target_lane = if nxt.end != 0 { left - 1 - r } else { r + left };
    }
}

/// The distance left to the lane's end, the junction there and which of its ways the car comes in on; the car enters
/// the intersection once within a unit of the end.
fn approach_junction(traffic: &mut Traffic, map: &StreetMap, id: usize, st: &MapStreet, to: Vec3) -> (f32, usize, i32) {
    let c = &mut traffic.cars[id];
    let cur = c.step(c.route_index);
    let d = if cur.end != 0 { -st.dir } else { st.dir };
    let (s, co) = sincos(c.yaw);
    let pos = c.pos;
    let remaining = (d.x * (to.x - (co * 4.0 + pos.x)) + d.y * 0.0) + d.z * (to.z - (s * 4.0 + pos.z));
    let junction = st.intersections[(cur.end ^ 1) as usize];
    let ways = map.intersections[junction].streets;
    let mut way = if ways[2] == cur.street { 2 } else { (ways[1] == cur.street) as i32 };

    if ways[3] == cur.street {
        way = 3;
    }

    if 1.0 > remaining {
        c.intersection = junction as i32;
        c.street = -1;
    }

    (remaining, junction, way)
}

/// The cruising speed (faster for an aggressive car far from a turn), cut to the stopping speed at a red or amber
/// light. Returns the stopping speed.
fn slow_for_signal(traffic: &mut Traffic, id: usize, remaining: f32, junction: usize, way: i32) -> f32 {
    let c = &traffic.cars[id];
    let next_turn = c.step(c.route_index + 1).turn;
    let aggressive = c.is_aggressive != 0;
    let fast = aggressive && (remaining > 40.0 || next_turn == 0);
    let base = if fast { coop_speed(traffic, 8) } else { 4.0 / 15.0 };
    let c = &mut traffic.cars[id];
    let mut approach = (((remaining - 4.0) * 0.5) / TICKS_PER_SECOND_F32).max(0.0);

    if remaining < 6.0 {
        approach = 0.0;
    }

    c.target_speed = ((id & 7) as f32 * 0.5) / TICKS_PER_SECOND_F32 + base;

    if !aggressive {
        let light = if next_turn == -1 { way + 4 } else { way };

        if traffic.signals.get(junction).is_some_and(|sg| sg.lights[light as usize] <= 1) && c.target_speed > approach {
            c.target_speed = approach;
        }
    }

    approach
}

/// Near the junction, the stopping speed while a nearly still car waits at the start of the target lane on the next
/// street. Returns whether the speed was cut.
fn slow_for_queue(traffic: &mut Traffic, id: usize, remaining: f32, mut approach: f32) -> bool {
    let c = &traffic.cars[id];
    let nxt = c.step(c.route_index + 1);

    if !(32.0 > remaining && nxt.street != -1) {
        return false;
    }

    let aggressive = c.is_aggressive != 0;
    let target_lane = c.target_lane;
    let Traffic { street_cars, cars, .. } = &mut *traffic;
    let list = street_cars.get(nxt.street as usize).map_or(&[][..], |v| v.as_slice());
    let mut limited = false;

    for &j in list {
        if j == id || (cars[j].lane_mask >> (target_lane & 31)) & 1 == 0 {
            continue;
        }

        if aggressive {
            approach = 0.2_f32.max(approach);
        }

        let o = &cars[j];

        if o.near[(nxt.end & 1) as usize] >= 12.0 || o.vel.length() >= 1.0 / 15.0 {
            continue;
        }

        let c = &mut cars[id];

        if c.target_speed > approach {
            c.target_speed = approach;
            limited = true;
        }
    }

    limited
}

/// The speed cut to the room left in the car's lanes, and in the street just ahead of it when it is about to leave
/// its own. Returns None when a calm car is done for the tick, else whether the car is held up (so it may try another
/// lane).
fn slow_for_clearance(traffic: &mut Traffic, map: &StreetMap, id: usize, st: &MapStreet, mut limited: bool) -> Option<bool> {
    let c = &traffic.cars[id];
    let aggressive = c.is_aggressive != 0;
    let (s, co) = sincos(c.yaw);
    let pos = c.pos;
    let look = Vec3::new(co * 12.0 + pos.x, pos.y + 0.0, pos.z + 12.0 * s);
    let ps = c.physical_street;
    let ahead = map
        .streets
        .iter()
        .enumerate()
        .rposition(|(k, o)| k as i32 != ps && look.cmpge(o.bounds_min).all() && o.bounds_max.cmpge(look).all())
        .map_or(-1, |k| k as i32);

    if ps != -1 {
        let sd = st.dir;
        let dot = ((co * sd.x) + sd.y * 0.0) + s * sd.z;
        let mask = (1 << (c.next_lane & 31)) | c.lane_mask;
        let mut clr = lane_clearance(traffic, map, id, ps, mask, 0.0 > dot, c.front_proj);

        if aggressive {
            clr = 0.2_f32.max(clr);
        }

        let c = &mut traffic.cars[id];

        if c.target_speed > clr {
            c.target_speed = clr;
            limited = true;

            if ahead == -1 {
                return aggressive.then_some(true);
            }
        }
    }

    if ahead == -1 {
        return aggressive.then_some(limited);
    }

    let o = &map.streets[ahead as usize];
    let d2 = o.dir;
    let w = map.intersections[o.intersections[0]].world_pos;
    let dot = (co * d2.x + d2.y * 0.0) + s * d2.z;
    let c = &traffic.cars[id];
    let (pmin, pmax) = ((c.bounds_min - w).dot(d2), (c.bounds_max - w).dot(d2));
    let p = if dot > 0.0 { pmin.max(pmax) } else { pmin.min(pmax) };
    let clr = lane_clearance(traffic, map, id, ahead, 1 << (c.next_lane & 31), 0.0 > dot, p);
    let c = &mut traffic.cars[id];

    if !aggressive {
        if c.target_speed > clr {
            c.target_speed = clr;
        }

        return None;
    }

    let clr = 0.2_f32.max(clr);

    if c.target_speed > clr {
        c.target_speed = clr;

        return Some(true);
    }

    Some(limited)
}

/// The intersection part of ai_traffic_car: a curve from the lane's end to the next street's lane, slower for a turn
/// and with the steering, taking the next street once its lane's start is passed.
fn cross_intersection(traffic: &mut Traffic, map: &StreetMap, id: usize) {
    let c = &traffic.cars[id];

    if c.intersection == -1 {
        return;
    }

    let ri = c.route_index;
    let (cur, nxt) = (c.step(ri), c.step(ri + 1));
    let st = &map.streets[cur.street as usize];
    let l0 = lane_at(st, c.lane);
    let p0 = if cur.end != 0 { l0.start } else { l0.end };
    let Some(nst) = map.streets.get(nxt.street as usize) else { return };
    let l1 = lane_at(nst, c.target_lane);
    let p1 = if nxt.end != 0 { l1.end } else { l1.start };
    let (ds, dn) = (st.dir, nst.dir);
    let delta = p1 - p0;
    let mut a = ds.dot(delta).abs();
    let b = dn.dot(ds).abs();

    if b > 0.5 {
        a *= 0.5;
    }

    if cur.end != 0 {
        a = -a;
    }

    let ctrl = ds * a + p0;
    let len = ((delta.x * delta.x + 0.0) + delta.z * delta.z).sqrt();

    let u = if len == 0.0 {
        Vec3::ZERO
    } else {
        let inv = 1.0 / len;
        Vec3::new(delta.x * inv, inv * 0.0, delta.z * inv)
    };

    let (s, co) = sincos(c.yaw);
    let pos = c.pos;

    let (t, om, q0, q1) = 'bez: {
        let clamped_end = (1.0, 0.0, p0 * 0.0 * 0.0, p1);

        if len <= BEZIER_EPSILON {
            break 'bez clamped_end;
        }

        let w = c.wheelbase + 4.0;
        let t = ((((w * 0.0 + pos.y) - p0.y) * u.y + ((co * w + pos.x) - p0.x) * u.x) + ((s * w + pos.z) - p0.z) * u.z) / len + 0.125;

        if 0.0 > t {
            (0.0, 1.0, p0, p1 * 0.0 * 0.0)
        } else if t > 1.0 {
            clamped_end
        } else {
            let om = 1.0 - t;
            (t, om, p0 * om * om, p1 * t * t)
        }
    };

    let b = q0 + (ctrl + ctrl) * om * t + q1 - pos;
    let dist = b.length();
    let target_yaw = if dist > BEZIER_EPSILON { (b.z as f64).atan2(b.x as f64) as f32 } else { c.yaw };
    let want = steer_toward(c.yaw, target_yaw, 56.25_f64.to_radians(), 56.25_f32.to_radians());
    let mut dsteer = want - c.steer;

    let factor = if -0.25 > dsteer {
        dsteer = -0.25;
        0.5
    } else if dsteer > 0.25 {
        dsteer = 0.25;
        0.5
    } else {
        let h = 0.5 - dsteer.abs() as f64;
        h + h
    };

    let aggressive = c.is_aggressive != 0;
    let k = c.step(ri + 1).turn;
    let mut base = 2.0 / 15.0;

    if k == 0 {
        base = 4.0 / 15.0;

        if aggressive {
            base = 0.4_f32 as f64;

            if traffic.coop {
                base = coop_speed(traffic, 4) as f64;
            }
        }
    }

    let c = &mut traffic.cars[id];
    c.steer += dsteer;
    c.target_speed = (factor * base) as f32;

    let dn2 = if c.step(ri + 1).end != 0 { -dn } else { dn };
    let qx = (p1.x - (dn2.x * 4.0 + pos.x)) * dn2.x;
    let qz = (p1.z - (4.0 * dn2.z + pos.z)) * dn2.z;
    let q = qz + (0.0 * dn2.y + qx);

    if 1.0 > q {
        if ri < c.route_len - 1 {
            c.route_index = ri + 1;
            c.intersection = -1;
            c.street = nxt.street;
            c.lane = c.target_lane;
            c.next_lane = c.target_lane;
        } else {
            c.intersection = -1;
            c.street = cur.street;
            let n = map.streets.len() as i32;

            if cur.street == LOOP_STREET {
                if aggressive && n != 0 {
                    random_street(map);
                }
            } else {
                let mut dest = LOOP_STREET;

                if aggressive {
                    dest = n;

                    if n != 0 {
                        dest = random_street(map);
                    }
                }

                if !((cur.street - LOOP_STREET) as u32 <= 1) {
                    let r = rand::random_range(0..2);
                    let c = &mut traffic.cars[id];
                    let e = c.step(c.route_index).end;

                    plan_route(c, map, cur.street, e, dest, r);
                }
            }
        }
    }

    if nxt.street == -1 {
        return;
    }

    let c = &traffic.cars[id];
    let e2 = c.step(c.route_index + 1).end;
    let w = map.intersections[nst.intersections[0]].world_pos;
    let (mn, mx) = (c.bounds_min, c.bounds_max);
    let pmax = ((mx.x - w.x) * dn.x + (mx.y - w.y) * dn.y) + (mx.z - w.z) * dn.z;
    let pmin = ((mn.x - w.x) * dn.x + (mn.y - w.y) * dn.y) + (mn.z - w.z) * dn.z;
    let p = if e2 != 0 { if pmax > pmin { pmin } else { pmax } } else { pmin.max(pmax) };
    let clr = lane_clearance(traffic, map, id, nxt.street, 1 << (c.target_lane & 31), e2 != 0, p);
    let c = &mut traffic.cars[id];

    if c.target_speed > clr {
        c.target_speed = clr;
    }
}
