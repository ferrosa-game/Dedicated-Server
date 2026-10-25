use ferrosa_wit::{Mat3, Player, server};

use super::{ITEM_LIFT, ahead};

pub fn spawn(p: &Player, args: &[&str]) {
    let number = args.first().and_then(|a| a.parse::<i32>().ok()).unwrap_or(9999);
    let Some(item) = server::spawn_item("phone", ahead(p) + ITEM_LIFT, Mat3::IDENTITY) else { return };

    item.set_phone_number(number);

    server::announce(&format!("Phone {number} spawned"));
}
