pub const NAMES: [&str; 40] = [
    "ATOMIC", "BRAZIL", "CROWN", "DENVER", "EAGLE", "FALCON", "GUILD", "HOUND", "ACTOR", "GOLD", "RIVER", "PIANO", "PILLOW", "ANIMAL", "HORSE", "HONEY",
    "BRANCH", "LAMP", "LION", "TENT", "LONDON", "NIGHT", "FISH", "FLOWER", "TRUCK", "GLASS", "GHOST", "KNIFE", "SPOON", "TRAFFIC", "VASE", "SUGAR", "KITCHEN",
    "SCOOTER", "KING", "QUEEN", "LIZARD", "CARAVAN", "DREAM", "DRESS",
];

pub const CORPORATION_NAMES: [&str; 6] = ["Goldmen", "Monsota", "OXS", "Nexaco", "Pentacom", "Prodocon"];

pub(super) fn pick_name(used: &mut Vec<bool>, take: bool) -> usize {
    if used.len() < 512 {
        used.resize(512, false);
    }

    let mut k = rand::random_range(0..32) as usize;
    let mut tries = 1024;

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

pub(super) fn pick_volume(vols: &[i32]) -> i32 {
    if vols.is_empty() {
        return -1;
    }

    vols[rand::random_range(0..vols.len())]
}

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
