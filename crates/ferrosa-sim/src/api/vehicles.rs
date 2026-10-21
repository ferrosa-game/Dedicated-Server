use ferrosa_physics::{RotMatrix, rotation::rotate};
use ferrosa_protocol::clientbound::game::VehicleKind;
use glam::Vec3;

use crate::{sim::Sim, vehicle::Vehicle, world::vehicle_types::model_name};

impl Sim {
    pub fn vehicle_kind_names(&self) -> Vec<String> {
        (0..VehicleKind::COUNT as u8).filter_map(|k| VehicleKind::try_from(k).ok()).map(model_name).collect()
    }

    pub fn find_vehicle_kind(&self, name: &str) -> Option<u32> {
        self.vehicle_kind_names().iter().position(|k| k.eq_ignore_ascii_case(name)).map(|k| k as u32)
    }

    pub fn vehicle_display_name(&self, kind: u32) -> Option<&str> {
        self.vehicle_types.get(kind as usize).filter(|t| !t.nodes.is_empty()).map(|t| t.name.as_str())
    }

    pub fn vehicle_price(&self, kind: u32) -> Option<i32> {
        self.vehicle_types.get(kind as usize).map(|t| t.price)
    }

    pub fn place_vehicle(&mut self, kind: u32, colour: i32, centre: Vec3, rot: RotMatrix) -> Option<usize> {
        let kind = VehicleKind::try_from(u8::try_from(kind).ok()?).ok()?;
        let com = self.vehicle_types.get(kind as usize).filter(|t| !t.nodes.is_empty())?.center_of_mass;

        self.spawn_vehicle(kind, colour, centre - rotate(&rot, com), rot)
    }

    pub fn vehicle_ids(&self) -> Vec<usize> {
        self.vehicles.ids()
    }

    pub fn vehicle(&self, id: usize) -> Option<&Vehicle> {
        self.vehicles.get(id)
    }

    pub fn vehicle_pos(&self, id: usize) -> Option<Vec3> {
        self.bodies.get(self.vehicles.get(id)?.body).map(|b| b.pos)
    }

    /// A town car on a random street with a route to another, at least `min_distance` away when given, spawned as a
    /// vehicle already moving: the bot in its driver's seat follows the route. None for a map without streets.
    pub fn spawn_route_car(&mut self, min_distance: Option<f32>) -> Option<usize> {
        if self.world.map.streets.streets.is_empty() {
            return None;
        }

        self.spawn_chase_car(min_distance).1
    }

    /// Makes a vehicle a parked traffic car, which bots can drive.
    pub fn park_in_traffic(&mut self, id: usize) {
        let Some(kind) = self.vehicles.get(id).map(|v| v.kind) else { return };
        let car = crate::traffic::spawn::create_traffic_car(&mut self.traffic, &self.world.map, &self.vehicle_types, kind, 0, 0, 0, 0, 0.0);

        self.traffic.cars[car].is_bot = 0;
        self.traffic.cars[car].vehicle = id as i32;

        if let Some(v) = self.vehicles.get_mut(id) {
            v.traffic_car = car as i32;
        }
    }

    pub fn remove_vehicle(&mut self, id: usize) {
        self.delete_vehicle(id);
    }
}
