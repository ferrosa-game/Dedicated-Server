use crate::codec::{WireWrite, Writer};

/// Event 5: an item made, removed or moved (the client applies it to its item).
#[derive(Clone, Debug, PartialEq)]
pub struct EventUpdateItem {
    pub item_id: i32,
    pub active: bool,
    /// The item type as sent (six bits).
    pub item_type: u8,
    pub parent_human: i32,
    pub parent_item: i32,
    pub parent_slot: i32,
}

impl WireWrite for EventUpdateItem {
    fn write(&self, w: &mut Writer) {
        w.bits(self.item_id | (self.active as i32) << 10 | (self.item_type as i32) << 11, 20);
        w.bits(self.parent_human, 10);
        w.bits(self.parent_item, 10);
        w.bits(self.parent_slot, 4);
    }
}

/// Event 17: a human made or removed, and its player.
#[derive(Clone, Debug, PartialEq)]
pub struct EventUpdateHuman {
    pub human_id: i32,
    pub active: bool,
    pub player_id: i32,
}

impl WireWrite for EventUpdateHuman {
    fn write(&self, w: &mut Writer) {
        w.bits(self.human_id, 10);
        w.bits(self.active as i32, 1);
        w.bits(self.player_id, 8);
    }
}

/// Event 18: a block cell's value set (area_set_block_cell_value_b).
#[derive(Clone, Debug, PartialEq)]
pub struct EventSetBlockCell {
    pub area: i32,
    pub block_x: i32,
    pub block_y: i32,
    pub block_z: i32,
    pub value: u32,
}

impl WireWrite for EventSetBlockCell {
    fn write(&self, w: &mut Writer) {
        w.i32(self.area + (self.block_x << 16));
        w.i32((self.block_z << 16) + self.block_y);
        w.u32(self.value);
    }
}

/// Events 11 and 14, which client_unpack_events reads and client_process_events does nothing with: 11 four
/// 32-bit values, 14 values of 8, 8, 8 and 20 bits.
#[derive(Clone, Debug, PartialEq)]
pub struct EventUnused {
    pub values: [i32; 4],
}
