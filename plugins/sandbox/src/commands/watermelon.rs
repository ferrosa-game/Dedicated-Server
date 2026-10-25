use ferrosa_wit::{Mat3, Player, Vec3, server};

use super::ahead;

pub fn spawn(p: &Player) {
    let Some(melon) = server::find_item_kind("watermelon") else { return };

    for _ in 0..100 {
        let offset = Vec3::new(rand::random_range(1.0..=5.0), rand::random_range(1.0..=5.0), 1.0 + rand::random_range(1.0..=5.0));
        server::spawn_item_by_id(melon, ahead(p) + offset, Mat3::IDENTITY);
    }
}
