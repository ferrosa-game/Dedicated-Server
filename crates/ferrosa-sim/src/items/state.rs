use ferrosa_protocol::clientbound::game::ItemKind;

use crate::{
    PlayerId,
    items::{logic::phone::Phone, types::ItemType}
};

/// The part of an item record that depends on its type. The binary keeps all of these in one record and reuses
/// fields across types (a radio's channel sits in the computer's line fields); here each type carries only its own.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemState {
    /// Nothing beyond the common item fields.
    Plain,
    /// Something that is used up (starting at the type's magazine_ammo): a magazine's or the Auto5's
    /// rounds, the soccer ball's charge.
    Stock { left: i32 },
    /// A gun: rounds ready to fire (the Auto5's own 300 shells, otherwise the round chambered from the
    /// magazine), ticks until it can fire again and how long the trigger has been held.
    Gun { rounds: i32, cooldown: i32, trigger_ticks: i32 },
    /// Uses left and how far the current patching has got (done past 255).
    Bandage { usage_left: i32, progress: i32 },
    Burger { bites_left: i32 },
    /// The pin (1 while in), the fuse (240 while held unpinned, counting down once thrown) and the
    /// player who pulled the pin.
    Grenade { pin: i32, fuse: i32, primer: Option<PlayerId> },
    /// Phones and pay phones.
    Phone(Phone),
    /// A walkie-talkie: its channel (low 7 bits) and whether the use key is held to talk.
    Radio { channel: i32, transmitting: bool },
    /// An arcade machine's screen: the frame counter and top line.
    Arcade { frame: i32, top_line: i32 },
    Key { vehicle: Option<usize> },
    /// A stack of world cash.
    Cash(Cash),
    /// A computer's screen, shell and programs.
    Computer(Box<crate::computer::Computer>),
    Memo(Box<[u8; 1024]>),
    /// A door on its hinges: its axis and the two points it hangs from.
    Door { axis: glam::Vec3, hinges: [glam::Vec3; 2] },
}

pub const BILL_VALUES: [i32; 8] = [1, 5, 10, 20, 50, 100, 1000, 0];

/// A stack of up to ten bills: one less than the count, which bill is picked out and each bill's
/// code, three bits apiece from the bottom.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Cash {
    pub spread: i32,
    pub bills: i32,
    pub codes: u32,
}

impl Cash {
    pub(crate) fn code(&self, k: i32) -> u32 {
        (self.codes >> ((k * 3) & 31)) & 7
    }

    /// value: calculate the value of this stack of cash from its bills
    pub(crate) fn value(&self) -> i32 {
        (0..=self.bills).map(|k| BILL_VALUES[self.code(k) as usize]).sum()
    }

    /// cash_separate: puts a bill of `code` in at `index` (on top past the end), moving the bills above it up; a
    /// stack holds ten.
    pub(crate) fn insert(&mut self, index: i32, code: u32) -> bool {
        let bills = self.bills;

        if bills > 8 {
            return false;
        }

        self.bills += 1;

        let mut v = self.codes;

        let pos = if bills < index {
            3 * (bills + 1)
        } else {
            let mut k = 3 * bills;

            loop {
                let mask = !(7u32 << ((k + 3) & 31)) & v;
                v = (((v >> (k & 31)) & 7) << ((k + 3) & 31)) | mask;
                k -= 3;

                if k == 3 * index - 3 {
                    break;
                }
            }

            3 * index
        };

        v = (v & !(7u32 << (pos & 31))) | (code << (pos & 31));

        self.spread += 1;
        self.codes = v;

        true
    }

    /// cash_combine: takes out bill `index`, moving the bills above it down. Returns false when it was the last
    /// bill, which leaves the stack to despawn.
    pub(crate) fn remove(&mut self, index: i32) -> bool {
        let bills = self.bills;

        if bills == 0 {
            return false;
        }

        if bills < 0 {
            return true;
        }

        self.bills -= 1;

        if bills - 1 < index {
            return true;
        }

        let (mut v, mut k) = (self.codes, 3 * index);

        loop {
            let mask = !(7u32 << (k & 31)) & v;
            v = (((mask >> ((k + 3) & 31)) & 7) << (k & 31)) | mask;
            k += 3;

            if k == 3 * bills {
                break;
            }
        }

        self.codes = v;

        true
    }
}

impl ItemState {
    pub(crate) fn new(kind: ItemKind, ty: &ItemType) -> Self {
        let left = ty.magazine_ammo;

        match kind {
            ItemKind::Bandage => Self::Bandage { usage_left: left, progress: 0 },
            ItemKind::Burger => Self::Burger { bites_left: left },
            ItemKind::Grenade => Self::Grenade { pin: left, fuse: 0, primer: None },
            ItemKind::Phone | ItemKind::PhonePay => Self::Phone(Phone::default()),
            ItemKind::Radio => Self::Radio { channel: 0, transmitting: false },
            ItemKind::Arcade => Self::Arcade { frame: 0, top_line: 0 },
            ItemKind::Key => Self::Key { vehicle: None },
            ItemKind::CashWorld => Self::Cash(Cash::default()),
            ItemKind::Paper | ItemKind::PaperWorld => Self::Memo(Box::new([0; 1024])),
            _ if ty.is_gun => Self::Gun { rounds: left, cooldown: 0, trigger_ticks: 0 },
            _ if left > 0 => Self::Stock { left },
            _ => Self::Plain,
        }
    }

    /// What is left of a used-up item, 0 for the rest.
    pub(crate) fn left(&self) -> i32 {
        match self {
            Self::Stock { left } | Self::Bandage { usage_left: left, .. } | Self::Burger { bites_left: left } => *left,
            Self::Gun { rounds, .. } => *rounds,
            Self::Grenade { pin, .. } => *pin,
            _ => 0,
        }
    }

    /// Sets what is left, for the states that keep it.
    pub(crate) fn set_left(&mut self, v: i32) {
        match self {
            Self::Stock { left } | Self::Bandage { usage_left: left, .. } | Self::Burger { bites_left: left } => *left = v,
            Self::Gun { rounds, .. } => *rounds = v,
            Self::Grenade { pin, .. } => *pin = v,
            _ => {}
        }
    }

    pub(crate) fn cash(&self) -> Option<&Cash> {
        if let Self::Cash(c) = self { Some(c) } else { None }
    }

    pub(crate) fn cash_mut(&mut self) -> Option<&mut Cash> {
        if let Self::Cash(c) = self { Some(c) } else { None }
    }

    pub(crate) fn phone(&self) -> Option<&Phone> {
        if let Self::Phone(p) = self { Some(p) } else { None }
    }

    pub(crate) fn phone_mut(&mut self) -> Option<&mut Phone> {
        if let Self::Phone(p) = self { Some(p) } else { None }
    }
}
