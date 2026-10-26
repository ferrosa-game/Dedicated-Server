use ferrosa_protocol::serverbound::game::InputFlags;

use crate::{human::Human, items::Item};

/// One of the client's keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    LeftMouse,
    RightMouse,
    /// Space
    Jump,
    /// C
    Crouch,
    Shift,
    /// Q
    ThrowItem,
    /// Any of the keys that switch what the held item does, which block using it.
    ModeKey,
    /// E
    Grab,
    /// R
    Reload,
    /// F
    SwitchHands,
    /// Go limp (movement state 2) while held.
    Ragdoll,
}

impl Input {
    /// The key's bits in the raw input flags.
    pub const fn mask(self) -> u32 {
        match self {
            Input::LeftMouse => InputFlags::LEFT_MOUSE.bits(),
            Input::RightMouse => InputFlags::RIGHT_MOUSE.bits(),
            Input::Jump => InputFlags::JUMP.bits(),
            Input::Crouch => InputFlags::CROUCH.bits(),
            Input::Shift => InputFlags::SHIFT.bits(),
            Input::ThrowItem => InputFlags::DROP_ITEM.bits(),
            Input::ModeKey => InputFlags::MODE_KEYS.bits(),
            Input::Grab => InputFlags::GRAB.bits(),
            Input::Reload => InputFlags::RELOAD.bits(),
            Input::SwitchHands => InputFlags::SWITCH_HANDS.bits(),
            Input::Ragdoll => InputFlags::RAGDOLL.bits(),
        }
    }
}

impl Human {
    /// Whether the key is held this tick.
    pub fn has_input(&self, input: Input) -> bool {
        self.input_flags & input.mask() != 0
    }

    /// Whether the key was held last tick.
    pub fn had_input(&self, input: Input) -> bool {
        self.last_input_flags & input.mask() != 0
    }

    /// Whether the key went down this tick.
    pub fn pressed(&self, input: Input) -> bool {
        self.has_input(input) && !self.had_input(input)
    }

    /// Whether the key was let go this tick.
    pub fn released(&self, input: Input) -> bool {
        !self.has_input(input) && self.had_input(input)
    }
}

/// What a human does with the item in its hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemInput {
    /// The item is being used (left mouse with this hand).
    Use,
    /// The item's second action (reload).
    Secondary,
}

impl ItemInput {
    /// The flag's bit in the item's input flags.
    pub const fn mask(self) -> u32 {
        match self {
            ItemInput::Use => 1,
            ItemInput::Secondary => 2,
        }
    }
}

impl Item {
    /// Whether the item is being used this way this tick.
    pub fn has_input(&self, input: ItemInput) -> bool {
        self.input_flags & input.mask() != 0
    }

    /// Whether the item was being used this way last tick.
    pub fn had_input(&self, input: ItemInput) -> bool {
        self.last_input_flags & input.mask() != 0
    }

    /// Whether this use started this tick.
    pub fn pressed(&self, input: ItemInput) -> bool {
        self.has_input(input) && !self.had_input(input)
    }
}
