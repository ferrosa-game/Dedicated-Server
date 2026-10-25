use ferrosa_wit::{Human, Player};

fn bandage(h: &Human) {
    h.set_bleeding(false);

    if h.vitality() <= 9 {
        h.set_vitality(10);
    }
}

pub fn heal(p: &Player) {
    p.human().into_iter().for_each(|h| bandage(&h));
}
