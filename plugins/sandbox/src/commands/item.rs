use ferrosa_wit::{Mat3, Player, server};

use super::{ITEM_LIFT, ahead};

pub fn spawn(p: &Player, args: &[&str]) {
    let kinds = server::item_kinds();
    let arg = args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let kind = kinds.iter().position(|k| *k == arg).or_else(|| arg.parse::<usize>().ok().filter(|&k| k < kinds.len()));

    match kind {
        Some(k) => _ = server::spawn_item_by_id(k as u32, ahead(p) + ITEM_LIFT, Mat3::IDENTITY),
        None => p.message(&format!("/item {}", kinds.join(" "))),
    }
}
