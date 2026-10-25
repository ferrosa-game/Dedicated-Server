use super::TrafficCar;
use crate::world::streets::StreetMap;

#[derive(Clone, Copy, Default)]
struct SearchNode {
    state: i32,
    cost: f32,
    parent: usize,
}

/// One step of the route search: the open node with the lowest cost plus straight distance to the goal is
/// closed and its neighbours opened; reaching the goal fills `path` from the goal back to the start.
fn search_step(map: &StreetMap, nodes: &mut [SearchNode], from: usize, to: usize, path: &mut Vec<usize>) {
    let goal = map.route_nodes[to].pos;
    let mut best = 65536.0;
    let mut sel = 0;

    for (i, n) in map.route_nodes.iter().enumerate() {
        if nodes[i].state != 1 {
            continue;
        }

        let d = goal - n.pos;
        let f = d.length() * (1.0 / 128.0) + nodes[i].cost;

        if best > f {
            sel = i;
            best = f;
        }
    }

    if nodes[sel].state != 1 {
        return;
    }

    nodes[sel].state = 2;

    for &j in &map.route_nodes[sel].neighbors {
        let j = j as usize;
        let cost = nodes[sel].cost + 1.0;
        let dead_end = map.route_nodes[j].dead_end != 0;

        match nodes[j].state {
            2 => continue,
            0 => {
                nodes[j] = SearchNode { state: 1, cost: if dead_end { cost + 256.0 } else { cost }, parent: sel };
            }
            _ => {
                if nodes[j].cost <= cost {
                    continue;
                }

                nodes[j].parent = sel;
                nodes[j].cost = if dead_end { cost + 256.0 } else { cost };
            }
        }

        if j == to {
            path.push(j);
            let mut cur = j;

            while cur != from {
                cur = nodes[cur].parent;
                path.push(cur);
            }

            return;
        }
    }
}

/// traffic_car_plan_route: the route from one end of a street to one end of another, as the steps the car drives and
/// the turn it takes onto each. Ends past the map's streets find no route (the binary searches whatever lies past the
/// node table), and a path is cut at the record's 63 steps (the binary writes on over the fields after them).
pub fn plan_route(car: &mut TrafficCar, map: &StreetMap, from_street: i32, from_slot: i32, to_street: i32, to_slot: i32) {
    let count = map.route_nodes.len();
    let (from, to) = ((from_street * 2 + from_slot) as usize, (to_street * 2 + to_slot) as usize);
    let mut path = Vec::new();

    if from < count && to < count {
        let mut nodes = vec![SearchNode::default(); count];
        nodes[from].state = 1;

        for _ in 0..4096 {
            search_step(map, &mut nodes, from, to, &mut path);

            if !path.is_empty() {
                break;
            }
        }
    }

    car.route_index = 0;
    car.route_len = 0;
    car.route[0].turn = 0;

    let path_len = path.len();

    if path_len == 0 {
        return;
    }

    let (mut from_way, mut to_way) = (0, 0);
    let mut street = map.route_nodes[path[path_len - 1]].street as i32;

    for path_step in 1..=path_len.min(63) {
        let end = map.route_nodes[path[path_len - path_step]].end;

        car.route[path_step - 1].street = street;
        car.route[path_step - 1].end = end;
        car.route_len = path_step as i32;

        if path_step == path_len || path_step == 63 {
            break;
        }

        let st = &map.streets[street as usize];
        let at = &map.intersections[st.intersections[if end != 0 { 0 } else { 1 }]];
        let next = map.route_nodes[path[path_len - 1 - path_step]].street as i32;

        for (k, &s) in at.streets.iter().enumerate() {
            if s == street {
                from_way = k as i32;
            }

            if s == next {
                to_way = k as i32;
            }
        }

        let mut diff = from_way - to_way;

        if diff < -1 {
            diff += 4;
        } else if diff > 1 {
            diff -= 4;
        }

        car.route[path_step].turn = if diff == -1 { -1 } else { (diff == 1) as i32 };
        street = next;
    }
}
