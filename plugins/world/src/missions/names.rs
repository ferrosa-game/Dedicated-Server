//! The project codenames, and picking one that is free today.

pub const NAMES: [&str; 40] = [
    "ATOMIC", "BRAZIL", "CROWN", "DENVER", "EAGLE", "FALCON", "GUILD", "HOUND", "ACTOR", "GOLD", "RIVER", "PIANO", "PILLOW", "ANIMAL", "HORSE", "HONEY",
    "BRANCH", "LAMP", "LION", "TENT", "LONDON", "NIGHT", "FISH", "FLOWER", "TRUCK", "GLASS", "GHOST", "KNIFE", "SPOON", "TRAFFIC", "VASE", "SUGAR", "KITCHEN",
    "SCOOTER", "KING", "QUEEN", "LIZARD", "CARAVAN", "DREAM", "DRESS",
];

/// The corporations' names, as their memos and status lines head them.
pub const CORPORATION_NAMES: [&str; 6] = ["Goldmen", "Monsota", "OXS", "Nexaco", "Pentacom", "Prodocon"];

pub(super) const PROJECTS: usize = 8;

pub(super) const NAME_FLAGS: usize = 512;

pub(super) const PICK_TRIES: i32 = 1024;

/// A name not yet taken today, tried at random up to 1024 times; `take` marks it taken.
pub(super) fn pick_name(used: &mut Vec<bool>, take: bool) -> usize {
    if used.len() < NAME_FLAGS {
        used.resize(NAME_FLAGS, false);
    }

    let mut k = rand::random_range(0..32) as usize;
    let mut tries = PICK_TRIES;

    while used[k] {
        k = rand::random_range(0..32) as usize;
        tries -= 1;

        if tries == 0 {
            break;
        }
    }

    if take {
        used[k] = true;
    }

    k
}

/// One of the volumes, at random.
pub(super) fn pick_volume(vols: &[i32]) -> i32 {
    if vols.is_empty() {
        return -1;
    }

    vols[rand::random_range(0..vols.len())]
}

/// The world clock as a 12 hour time: the hour, the minute and whether it is afternoon.
pub fn clock_time(time: i32) -> (i32, i32, bool) {
    let minutes = time / 60;
    let (h, m) = (minutes / 60 % 24, minutes % 60);
    let pm = h >= 12;
    let hour = match h % 12 {
        0 => 12,
        n => n,
    };

    (hour, m, pm)
}
