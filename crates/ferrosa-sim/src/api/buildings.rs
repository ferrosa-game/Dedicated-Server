use ferrosa_world::building::{BuildingRecord, PatrolSpot, ShopEntry};
use glam::{IVec3, Vec3};

use crate::sim::Sim;

impl Sim {
    pub fn buildings(&self) -> &[BuildingRecord] {
        &self.world.map.level.buildings
    }

    pub fn building(&self, id: usize) -> Option<&BuildingRecord> {
        self.world.map.level.buildings.get(id)
    }

    pub fn building_contains(&self, id: usize, pos: Vec3, margin: f32) -> bool {
        self.building(id).is_some_and(|b| b.contains(pos, margin))
    }

    pub fn building_shop(&self, id: usize) -> &[ShopEntry] {
        self.building(id).map_or(&[], |b| &b.shop)
    }

    pub fn set_building_shop(&mut self, id: usize, entries: Vec<ShopEntry>) {
        if let Some(b) = self.world.map.level.buildings.get_mut(id) {
            b.shop = entries;
        }
    }

    pub fn building_car_counter(&self, id: usize) -> i32 {
        self.building(id).map_or(0, |b| b.car_counter)
    }

    pub fn set_building_car_counter(&mut self, id: usize, next: i32) {
        if let Some(b) = self.world.map.level.buildings.get_mut(id) {
            b.car_counter = next;
        }
    }

    pub fn building_patrol(&self, id: usize) -> &[PatrolSpot] {
        self.building(id).map_or(&[], |b| &b.patrol)
    }

    pub fn building_computers(&self, id: usize) -> &[(IVec3, i32)] {
        self.building(id).map_or(&[], |b| &b.computers)
    }

    pub fn set_building_computer_volume(&mut self, id: usize, index: usize, volume: i32) {
        if let Some(c) = self.world.map.level.buildings.get_mut(id).and_then(|b| b.computers.get_mut(index)) {
            c.1 = volume;
        }
    }
}
