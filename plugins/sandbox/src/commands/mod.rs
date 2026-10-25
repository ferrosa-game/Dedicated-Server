use ferrosa_wit::{Player, Vec3};

mod car;
mod clear;
mod computer;
mod godmode;
mod gravity;
mod guns;
mod heal;
mod human;
mod item;
mod kill;
mod phone;
mod stocks;
mod time;
mod watermelon;

pub const ITEM_LIFT: Vec3 = Vec3::new(0.0, 0.0, 1.0);

pub fn ahead(p: &Player) -> Vec3 {
    p.camera() - p.view().x * 1.5
}

/// Runs the named command; false for a name the sandbox does not own, which the server then answers for.
pub fn run(p: &Player, name: &str, args: &[&str]) -> bool {
    match name {
        "human" => human::spawn(p),
        "kill" => kill::nearest(p),
        "heal" => heal::heal(p),
        "godmode" => godmode::toggle(p),
        "guns" => guns::spawn(p),
        "watermelon" => watermelon::spawn(p),
        "item" => item::spawn(p, args),
        "car" => car::spawn(p, args),
        "phone" => phone::spawn(p, args),
        "computer" => computer::spawn(p),
        "gravity" => gravity::set(args),
        "time" => time::set(args),
        "stocks" => stocks::toggle(p),
        "clear" => clear::everything(),
        _ => return false,
    }

    true
}
