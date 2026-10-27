use ferrosa_physics::RotMatrix;
use ferrosa_protocol::clientbound::game::ItemKind;
use glam::Vec3;

use crate::{
    items::{Item, state::ItemState},
    sim::Sim,
};

impl Sim {
    pub fn item_kind_names(&self) -> Vec<String> {
        (0..ItemKind::COUNT as u8).filter_map(|k| ItemKind::try_from(k).ok()).map(|k| format!("{k:?}").to_lowercase()).collect()
    }

    pub fn find_item_kind(&self, name: &str) -> Option<u32> {
        self.item_kind_names().iter().position(|k| k.eq_ignore_ascii_case(name)).map(|k| k as u32)
    }

    pub fn item_price(&self, kind: u32) -> Option<i32> {
        self.item_types.get(kind as usize).map(|t| t.price)
    }

    pub fn item_magazine_ammo(&self, kind: u32) -> Option<i32> {
        self.item_types.get(kind as usize).map(|t| t.magazine_ammo)
    }

    pub fn spawn_item(&mut self, kind: u32, pos: Vec3, rot: RotMatrix) -> Option<usize> {
        let kind = ItemKind::try_from(u8::try_from(kind).ok()?).ok()?;
        self.create_item(kind, pos, None, rot)
    }

    pub fn item_ids(&self) -> Vec<usize> {
        self.items.ids()
    }

    pub fn item(&self, id: usize) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn remove_item(&mut self, id: usize) {
        self.delete_item(id);
    }

    pub fn item_holder(&self, item: usize) -> Option<usize> {
        usize::try_from(self.items.get(item)?.parent_human).ok().filter(|&h| self.humans.get(h).is_some())
    }

    pub fn item_volume(&self, item: usize) -> Option<i32> {
        let volume = self.items.get(item)?.volume;

        self.fs.volume(volume).map(|_| volume)
    }

    pub fn set_item_volume(&mut self, item: usize, volume: i32) -> bool {
        if self.fs.volume(volume).is_none() {
            return false;
        }

        let Some(i) = self.items.get_mut(item) else { return false };

        if i.item_type != ItemKind::Computer && !i.item_type.is_disk() {
            return false;
        }

        i.volume = volume;

        true
    }

    pub fn item_children(&self, item: usize) -> Vec<usize> {
        self.items.get(item).map_or_else(Vec::new, |i| i.children.clone())
    }

    pub fn attach_item(&mut self, parent: usize, child: usize) -> bool {
        let closed = self.items.get(parent).is_some_and(|i| i.item_type == ItemKind::Briefcase);

        if closed {
            self.items.get_mut(parent).unwrap().item_type = ItemKind::BriefcaseOpen;
        }

        let attached = crate::items::attach_child(&mut self.items, &self.item_types, parent, child);

        if closed {
            self.items.get_mut(parent).unwrap().item_type = ItemKind::Briefcase;
        }

        attached
    }

    pub fn item_uses(&self, item: usize) -> i32 {
        self.items.get(item).map_or(0, |i| i.state.left())
    }

    pub fn set_item_uses(&mut self, item: usize, uses: i32) {
        if let Some(i) = self.items.get_mut(item) {
            i.state.set_left(uses);
        }
    }

    pub fn phone_number(&self, item: usize) -> Option<i32> {
        self.items.get(item).and_then(|i| i.state.phone()).map(|p| p.number)
    }

    pub fn set_phone_number(&mut self, item: usize, number: i32) -> bool {
        let Some(phone) = self.items.get_mut(item).and_then(|i| i.state.phone_mut()) else { return false };
        phone.number = number;
        true
    }

    pub fn set_phone_texture(&mut self, item: usize, texture: i32) -> bool {
        let Some(phone) = self.items.get_mut(item).and_then(|i| i.state.phone_mut()) else { return false };
        phone.texture = texture;

        self.phone_update(item);
        true
    }

    pub fn key_vehicle(&self, item: usize) -> Option<usize> {
        match self.items.get(item)?.state {
            ItemState::Key { vehicle } => vehicle,
            _ => None,
        }
    }

    pub fn set_key_vehicle(&mut self, item: usize, vehicle: Option<usize>) -> bool {
        let Some(ItemState::Key { vehicle: v }) = self.items.get_mut(item).map(|i| &mut i.state) else { return false };
        *v = vehicle;
        true
    }

    pub fn item_cash(&self, item: usize) -> Option<(i32, u32)> {
        self.items.get(item)?.state.cash().map(|c| (c.bills, c.codes))
    }

    pub fn set_item_cash(&mut self, item: usize, bills: i32, codes: u32) -> bool {
        let Some(c) = self.items.get_mut(item).and_then(|i| i.state.cash_mut()) else { return false };
        c.bills = bills;
        c.codes = codes;
        true
    }

    pub fn item_cash_value(&self, item: usize) -> Option<i32> {
        self.items.get(item)?.state.cash().map(|c| c.value())
    }

    pub fn insert_item_bill(&mut self, item: usize, at: i32, code: u32) -> bool {
        let Some(c) = self.items.get_mut(item).and_then(|i| i.state.cash_mut()) else { return false };

        c.insert(at, code)
    }

    pub fn item_is_static(&self, item: usize) -> bool {
        self.items.get(item).is_some_and(|i| i.is_static)
    }

    pub fn set_item_is_static(&mut self, item: usize, still: bool) {
        if let Some(i) = self.items.get_mut(item) {
            i.is_static = still;
        }
    }

    pub fn bill_value(code: u32) -> i32 {
        crate::items::state::BILL_VALUES.get(code as usize).copied().unwrap_or(0)
    }
}
