use ferrosa_physics::RotMatrix;
use ferrosa_protocol::clientbound::game::ItemKind;
use glam::Vec3;

use crate::{PlayerId, human::Human, sim::Sim};

impl Sim {
    pub fn human_ids(&self) -> Vec<usize> {
        self.humans.ids()
    }

    pub fn human(&self, id: usize) -> Option<&Human> {
        self.humans.get(id)
    }

    pub fn human_mut(&mut self, id: usize) -> Option<&mut Human> {
        self.humans.get_mut(id)
    }

    pub fn place_human(&mut self, pos: Vec3, rot: RotMatrix, player: Option<PlayerId>) -> Option<usize> {
        self.spawn_human(pos, &rot, player)
    }

    pub fn remove_human(&mut self, id: usize) {
        self.delete_human(id);
    }

    /// A hit of `damage` on one of a human's bones, as a bullet's.
    pub fn damage_human(&mut self, human: usize, bone: crate::world::skeleton::BoneId, damage: i32) {
        if let Some(h) = self.humans.get_mut(human) {
            crate::human::damage::damage_human(h, bone, damage);
        }
    }

    /// Puts a human in one of a vehicle's seats, 0 the driver's.
    pub fn enter_vehicle(&mut self, human: usize, vehicle: usize, seat: usize) -> bool {
        if self.vehicles.get(vehicle).is_none() {
            return false;
        }

        let Some(h) = self.humans.get_mut(human) else { return false };
        h.vehicle = Some(vehicle);
        h.seat = seat;
        true
    }

    pub fn set_view_yaw(&mut self, human: usize, yaw: f32) {
        if let Some(h) = self.humans.get_mut(human) {
            h.view_yaw = yaw;
        }
    }

    /// An item into the first free slot of `slots` of a human.
    pub fn give_human_item(&mut self, human: usize, kind: u32, slots: std::ops::Range<usize>) -> Option<usize> {
        let kind = u8::try_from(kind).ok().filter(|&k| ItemKind::try_from(k).is_ok())?;
        self.give_item(human, kind, slots, |_| {})
    }

    pub fn human_slot_items(&self, human: usize, slot: usize) -> Vec<usize> {
        let Some(s) = self.humans.get(human).and_then(|h| h.inventory.get(slot)) else { return Vec::new() };

        s.items[..s.count.clamp(0, s.items.len() as i32) as usize].iter().filter_map(|&i| usize::try_from(i).ok()).collect()
    }

    /// Puts an item into one of a human's slots.
    pub fn put_item(&mut self, human: usize, item: usize, slot: usize) {
        if slot < crate::human::INVENTORY_SLOTS && self.items.get(item).is_some() {
            self.link_to_slot(human, item, slot);
        }
    }
}
