use ferrosa_physics::{body::SolverStep, rotation::rot_matrix_to_quaternion};
use ferrosa_protocol::clientbound::game::{
    ServerVehicleObject,
    events::{Event, ServerEvent, update_vehicle::EventUpdateVehicle, update_vehicle_type_color::EventUpdateVehicleTypeColor},
};
use glam::Vec3;

use super::Sim;
use crate::{
    items::state::ItemState,
    vehicle::{physics, spawn_vehicle},
};

/// What a vehicle's priority is worked out from: its position, steering input, velocity and whether it is a train.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VehicleSight {
    pos: Vec3,
    steer_control: f32,
    vel: Vec3,
    train: bool,
}

/// The vehicle part of append_object_packet for one connection: every vehicle's priority grown, then the most overdue
/// sent with their tiers.
pub(crate) fn vehicle_section(
    priority: &mut Vec<i32>,
    vehicles: &[(ServerVehicleObject, VehicleSight)],
    camera: Vec3,
    backlog: u16,
) -> Vec<ServerVehicleObject> {
    priority.resize(128, 0);

    for (o, s) in vehicles {
        let d = camera - s.pos;
        let dist = d.length();
        let near = (dist as f64 / ((s.steer_control.abs() as f64) * 4.0 + 1.0)) as f32;
        let speed = s.vel.length();
        let k = if 1.0 / 60.0 <= speed { 5.0 } else { speed * 60.0 * 4.0 + 1.0 };
        let mut p = (((1.0 - (near / k) * (1.0 / 256.0)) * 256.0) as i32).max(32);

        if dist < 4.0 || (s.train && dist < 12.0) {
            p = 1024;
        }

        priority[o.vehicle_id as usize] += p;
    }

    let mut order: Vec<usize> = (0..vehicles.len()).collect();
    order.sort_by_key(|&i| priority[vehicles[i].0.vehicle_id as usize]);

    let sent: Vec<ServerVehicleObject> = order
        .into_iter()
        .rev()
        .take(16)
        .enumerate()
        .map(|(rank, i)| {
            let o = &vehicles[i].0;
            let tier = if rank < 2 {
                0
            } else if rank < 8 {
                1
            } else {
                2
            };

            if tier < 2 {
                priority[o.vehicle_id as usize] = 0;
            }

            ServerVehicleObject { tier, ..o.clone() }
        })
        .collect();

    let cap = if backlog > 22 { 2 } else { 16 };
    sent.into_iter().take(cap).collect()
}

fn to_byte(v: f32, min: i32) -> i32 {
    ((v * 255.0) as i32).clamp(min, 255)
}

impl Sim {
    /// spawn_vehicle and its update_vehicle_type_and_color event.
    pub(crate) fn spawn_vehicle(
        &mut self,
        kind: ferrosa_protocol::clientbound::game::VehicleKind,
        color: i32,
        pos: Vec3,
        rot: ferrosa_physics::RotMatrix,
    ) -> Option<usize> {
        let id = spawn_vehicle(&mut self.vehicles, &mut self.bodies, &self.vehicle_types, kind, color, pos, rot, None)?;
        let e = EventUpdateVehicleTypeColor { vehicle_id: id as i32, vehicle_type: kind, vehicle_color: color as u8 };

        self.events.push(Event { tick_created: self.tick, kind: ServerEvent::UpdateVehicleTypeColor(e) });

        Some(id)
    }

    /// The vehicle part of build_objectpacket: each vehicle's position, orientation, steering, wheels and engine, and
    /// what its priority for each connection is worked out from. A helicopter's rotor angles ride in its first two
    /// wheel slots' heights, where the binary keeps them (+0x3988, +0x3a34).
    pub(crate) fn vehicle_objects(&self) -> Vec<(ServerVehicleObject, VehicleSight)> {
        self.vehicles
            .iter()
            .filter(|(id, v)| *id < 128 && v.controllable_state != 3)
            .map(|(id, v)| {
                let steer = ((v.steer as f64 / 180.0_f64.to_radians() * 255.0) as i32).clamp(-255, 255);
                let wheels = std::array::from_fn(|k| match v.rotor.as_ref().and_then(|r| r.angles.get(k)) {
                    Some(&angle) => [to_byte(angle, 0), 0, 0],
                    None => v
                        .wheels
                        .get(k)
                        .filter(|_| k < 4)
                        .map_or([0; 3], |w| [to_byte(w.visual_height, 0), to_byte(w.spin, -255), to_byte(w.skid, 0)]),
                });
                let object = ServerVehicleObject {
                    vehicle_id: id as u16,
                    traffic_car: v.traffic_car,
                    pos: v.pos,
                    rot: rot_matrix_to_quaternion(&v.rot),
                    steer,
                    wheels,
                    engine_rpm: v.engine_rpm,
                    tier: 0,
                };
                (
                    object,
                    VehicleSight {
                        pos: v.pos,
                        steer_control: v.steer_control,
                        vel: v.vel,
                        train: v.train,
                    },
                )
            })
            .collect()
    }
}

impl Sim {
    /// vehicle_update_bbox_wake_items, after the bodies move: each vehicle's record, then the set items in the
    /// blocks its bounds cover.
    pub(crate) fn update_vehicle_bounds(&mut self) {
        for id in self.vehicles.ids() {
            let Some(v) = self.vehicles.get_mut(id) else { continue };

            if self.bodies.get(v.body).is_none() {
                continue;
            }

            physics::update_vehicle_bounds(v, &self.bodies, &self.vehicle_types);
            let (lo, hi) = (glam::IVec3::from_array(v.block_min), glam::IVec3::from_array(v.block_max));

            self.spawn_set_items_in(lo, hi);
        }
    }

    /// vehicleSimulation, with its crash sounds and broken glass as events.
    pub(crate) fn simulate_vehicles(&mut self) {
        use crate::{human::physics::HumanOutput, vehicle::physics::VehicleOutput};
        let driven: Vec<usize> = self.humans.iter().filter(|(_, h)| h.seat == 0).filter_map(|(_, h)| h.vehicle).collect();
        let (out, glass) = physics::vehicle_simulation(&mut self.vehicles, &mut self.bodies, &mut self.world.map, &self.vehicle_types, &driven);

        self.vehicle_item_contacts();

        physics::vehicle_vehicle_contacts(&self.vehicles, &mut self.bodies, &self.vehicle_types);

        for o in out {
            match o {
                VehicleOutput::Sound { sound, pos, volume, pitch } => self.apply_human_outputs(vec![HumanOutput::Sound { sound, pos, volume, pitch }]),
                VehicleOutput::Damage { vehicle, amount } => self.vehicle_take_damage(vehicle, amount),
                VehicleOutput::Update { vehicle, kind, part, pos, vel } => {
                    let e = EventUpdateVehicle { vehicle_id: vehicle as i32, kind, part, pos, velocity: vel };
                    self.events.push(Event { tick_created: self.tick, kind: ServerEvent::UpdateVehicle(e) });
                }
            }
        }

        self.apply_human_outputs(glass);
    }

    /// vehicle_take_damage: a vehicle's health worn down; at nothing it is wrecked (update_vehicle kind 2). Trains
    /// cannot be hurt.
    pub(crate) fn vehicle_take_damage(&mut self, id: usize, amount: i32) {
        let Some(vehicle) = self.vehicles.get_mut(id) else { return };
        
        if vehicle.train || vehicle.health <= 0 {
            return;
        }

        vehicle.health -= amount;

        if vehicle.health <= 0 {
            let event = EventUpdateVehicle { vehicle_id: id as i32, kind: 2, part: 0, pos: vehicle.pos, velocity: vehicle.vel };

            self.events.push(Event { tick_created: self.tick, kind: ServerEvent::UpdateVehicle(event) });
        }
    }

    /// delete_vehicle: anyone inside is let go, the chassis and wheel bodies removed and the clients told (update_vehicle
    /// kind 3).
    pub(crate) fn delete_vehicle(&mut self, id: usize) {
        for (_, human) in self.humans.iter_mut() {
            if human.vehicle == Some(id) {
                human.vehicle = None;
            }
        }

        for car in &mut self.traffic.cars {
            if car.vehicle == id as i32 {
                car.vehicle = -1;
            }
        }

        for (_, item) in self.items.iter_mut() {
            if let ItemState::Key { vehicle } = &mut item.state
                && *vehicle == Some(id)
            {
                item.despawn_time = 0;
                *vehicle = None;
            }
        }

        let Some(vehicle) = self.vehicles.remove(id) else { return };

        for w in &vehicle.wheels {
            self.bodies.remove(w.body);
        }

        if let Some(r) = &vehicle.rotor {
            self.bodies.remove_bond(r.point_bond);
            self.bodies.remove_bond(r.angular_bond);
            self.bodies.remove(r.body);
        }

        for &(bogie, turn) in &vehicle.bogies {
            self.bodies.remove_bond(turn);
            self.bodies.remove(bogie);
        }

        self.bodies.remove(vehicle.body);

        let e = EventUpdateVehicle { vehicle_id: id as i32, kind: 3, part: 0, pos: vehicle.pos, velocity: Vec3::ZERO };
        self.events.push(Event { tick_created: self.tick, kind: ServerEvent::UpdateVehicle(e) });
    }

    /// cleanup_vehicles: each vehicle's despawn time runs down and the vehicle is deleted when it runs out.
    pub(crate) fn cleanup_vehicles(&mut self) {
        for id in self.vehicles.ids() {
            let Some(vehicle) = self.vehicles.get_mut(id) else { continue };
            let t = vehicle.despawn_time as i32;

            if t > 65534 {
                continue;
            }

            vehicle.despawn_time = (t - 1) as u16;

            if t - 1 <= 0 {
                self.delete_vehicle(id);
            }
        }
    }

    fn vehicle_item_contacts(&mut self) {
        for (_, v) in self.vehicles.iter() {
            let Some(hull) = self.vehicle_types.get(v.kind as usize).and_then(|t| t.mesh.as_ref()) else { continue };
            let found = self.item_grid.query(glam::IVec3::from_array(v.block_min), glam::IVec3::from_array(v.block_max));

            for id in found {
                let Some(item) = self.items.get(id) else { continue };
                let (mn, mx) = (item.aabb_min, item.aabb_max);

                if mn.x > v.bounds_max.x
                    || v.bounds_min.x > mx.x
                    || mn.z > v.bounds_max.z
                    || v.bounds_min.z > mx.z
                    || mn.y > v.bounds_max.y
                    || v.bounds_min.y > mx.y
                {
                    continue;
                }

                let Some(item_hull) = &self.item_types[item.item_type as usize].hull else { continue };
                let item_body = item.body;
                let Some((ipos, irot)) = self.bodies.get(item_body).map(|b| (b.pos, b.rot)) else { continue };
                let contacts = super::hull::collide_convex_hulls(hull, v.pos, &v.rot, item_hull, ipos, &irot);

                for c in &contacts {
                    let (first, second) = if c.swapped { (item_body, v.body) } else { (v.body, item_body) };

                    self.bodies.add_body_contact(first, second, c.offset_a, c.offset_b, c.normal, c.depth, 0.4, 1.0 / 64.0, 1.0 / 32.0);
                }

                if !contacts.is_empty()
                    && let Some(item) = self.items.get_mut(id)
                {
                    item.physics_settled = false;
                    item.settled_timer = 0;
                }
            }
        }
    }

    pub(crate) fn solve_bonds(&mut self) {
        let Sim { bodies, vehicles, .. } = self;

        bodies.solve_bonds_with(|b, step| match step {
            SolverStep::BeforeBonds => physics::step_wheel_constraints(vehicles, b),
            SolverStep::AfterBonds => physics::apply_wheel_forces(vehicles, b),
        });
    }
}
