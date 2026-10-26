use ferrosa_world::building::{BuildingRecord, PatrolSpot, ShopEntry};
use glam::{IVec3, Vec3};

use crate::sim::Sim;

impl Sim {
    pub fn buildings(&self) -> &[BuildingRecord] {
        &self.world.map.level.buildings
    }

    /// The map's name, as its folder is called.
    pub fn map_name(&self) -> &str {
        &self.world.map.map_name
    }

    /// A street's name; empty for a street the map hasn't got.
    pub fn street_of(&self, street: i32) -> String {
        usize::try_from(street).ok().and_then(|s| self.world.map.streets.streets.get(s)).map_or(String::new(), |s| s.name.to_string())
    }

    /// The two streets of the intersection a street starts at, for naming a place: none when the corner has fewer
    /// than two.
    pub fn street_corner(&self, street: i32) -> Option<(String, String)> {
        let streets = &self.world.map.streets;
        let s = streets.streets.get(usize::try_from(street).ok()?)?;
        let i = streets.intersections.get(s.intersections[0])?;
        let a = if i.streets[0] != -1 { i.streets[0] } else { i.streets[2] };
        let b = if i.streets[1] != -1 { i.streets[1] } else { i.streets[3] };

        (a != -1 && b != -1).then(|| (self.street_of(a), self.street_of(b)))
    }

    /// The two intersections a street runs between, as numbers into `intersections`.
    pub fn street_intersections(&self, street: i32) -> Option<(u32, u32)> {
        let s = self.world.map.streets.streets.get(usize::try_from(street).ok()?)?;

        Some((s.intersections[0] as u32, s.intersections[1] as u32))
    }

    /// Where the trains start on the map's track, each with the way it faces.
    pub fn track_spawns(&self) -> &[(glam::Vec3, f32)] {
        &self.world.map.level.area.track.spawns
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
