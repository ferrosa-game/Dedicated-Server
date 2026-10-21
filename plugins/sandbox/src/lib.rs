use ferrosa_wit::{ChatType, ClientMode, GameState, Human, Mat3, Menu, Player, PlayerId, Plugin, Vec3, command, server};

const EARTH_GRAVITY: f32 = 9.8;
const ITEM_LIFT: Vec3 = Vec3::new(0.0, 0.0, 1.0);

const GUNS: [(&str, Option<&str>); 7] = [
    ("ak47", Some("ak47mag")),
    ("m16", Some("m16mag")),
    ("mp5", Some("mp5mag")),
    ("uzi", Some("uzimag")),
    ("pistol", Some("pistolmag")),
    ("magnum", Some("magnummag")),
    ("auto5", None)
];

#[derive(Default)]
struct Sandbox;

impl Plugin for Sandbox {
    fn init(&mut self) {
        server::register_mode("sandbox", &[], ClientMode::Round, None);
    }

    fn reset(&mut self) {
        server::set_game_state(GameState::InGame);
        server::set_traffic(128, false);
    }

    fn on_join(&mut self, player: PlayerId) {
        if let Some(p) = server::get_player(player) {
            p.message("Welcome to sandbox.");
        }
    }

    fn on_chat(&mut self, player: PlayerId, message: &str) -> bool {
        let Some((name, args)) = command(message) else { return false };
        let Some(p) = server::get_player(player) else { return false };

        match name {
            "human" => {
                if let Some(h) = server::spawn_human(p.camera(), Mat3::IDENTITY, Some(p.id())) {
                    p.set_human(&h);
                    p.set_menu(Menu::Empty);
                }
            }
            "kill" => kill_nearest(&p),
            "heal" => p.human().into_iter().for_each(|h| bandage(&h)),
            "godmode" => {
                let on = !p.god_mode();
                p.set_god_mode(on);
                server::send_chat(ChatType::AdminChat, -1, 0, if on { "godmode on" } else { "godmode off" });
            }
            "guns" => guns(&p),
            "watermelon" => watermelons(&p),
            "item" => item(&p, &args),
            "car" => car(&p, &args),
            "phone" => phone(&p, &args),
            "gravity" => gravity(&args),
            "stocks" => p.set_menu(if p.menu() == Menu::RoundCorpStock { Menu::Empty } else { Menu::RoundCorpStock }),
            "clear" => {
                server::items().iter().for_each(|i| i.delete());
                server::vehicles().iter().for_each(|v| v.delete());
                server::humans().iter().filter(|h| h.player().is_none()).for_each(|h| h.delete());
                server::clear_bullets();

                server::announce("Cleared");
            }
            _ => return false,
        }

        true
    }
}

fn kill_nearest(p: &Player) {
    let camera = p.camera();
    let nearest = server::humans().into_iter().filter(|h| h.vitality() > 0).min_by(|a, b| (a.pos() - camera).length().total_cmp(&(b.pos() - camera).length()));

    match nearest {
        Some(h) => h.set_vitality(0),
        None => p.message("No living human to kill"),
    }
}

fn bandage(h: &Human) {
    h.set_bleeding(false);

    if h.vitality() <= 9 {
        h.set_vitality(10);
    }
}

fn ahead(p: &Player) -> Vec3 {
    p.camera() - p.view().x * 1.5
}

fn guns(p: &Player) {
    let facing = p.facing();
    let (forward, right) = (-facing.x, -facing.z);
    let centre = p.camera() + forward * 1.5;
    let first = -(GUNS.len() as f32 - 1.0) * 0.5;

    for (k, (gun, mag)) in GUNS.into_iter().enumerate() {
        let at = centre + right * ((first + k as f32) * 0.75);

        server::spawn_item(gun, at, facing);

        let Some(mag) = mag else { continue };

        for m in 0..3 {
            server::spawn_item(mag, at + forward * (0.25 * (m as f32 + 1.0)), facing);
        }
    }
}

fn watermelons(p: &Player) {
    let Some(melon) = server::find_item_kind("watermelon") else { return };

    for _ in 0..100 {
        let offset = Vec3::new(rand::random_range(1.0..=5.0), rand::random_range(1.0..=5.0), 1.0 + rand::random_range(1.0..=5.0));
        server::spawn_item_by_id(melon, ahead(p) + offset, Mat3::IDENTITY);
    }
}

fn item(p: &Player, args: &[&str]) {
    let kinds = server::item_kinds();
    let arg = args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let kind = kinds.iter().position(|k| *k == arg).or_else(|| arg.parse::<usize>().ok().filter(|&k| k < kinds.len()));

    match kind {
        Some(k) => _ = server::spawn_item_by_id(k as u32, ahead(p) + ITEM_LIFT, Mat3::IDENTITY),
        None => p.message(&format!("/item {}", kinds.join(" "))),
    }
}

fn car(p: &Player, args: &[&str]) {
    let kinds = server::vehicle_kinds();
    let arg = args.first().copied().unwrap_or_default();
    let kind = server::find_vehicle_kind(arg).or_else(|| arg.parse::<u32>().ok().filter(|&k| (k as usize) < kinds.len()));
    let colour = args.get(1).and_then(|a| a.parse::<i32>().ok()).unwrap_or(0);

    if let Some(kind) = kind.filter(|&k| server::vehicle_display_name(k).is_some()) {
        let facing = p.facing();
        let spot = p.human().map_or(p.camera(), |h| h.pos()) - facing.z * 6.0;
        let spot = server::ground_below(spot).map_or(spot, |y| Vec3::new(spot.x, y + 1.0, spot.z));

        server::spawn_vehicle_by_id(kind, colour, spot, Mat3 { x: -facing.x, y: facing.y, z: -facing.z });
        return;
    }

    let spawnable: Vec<&str> = kinds.iter().enumerate().filter(|&(k, _)| server::vehicle_display_name(k as u32).is_some()).map(|(_, n)| n.as_str()).collect();
    p.message(&format!("/car {}", spawnable.join(" ")));
}

fn phone(p: &Player, args: &[&str]) {
    let number = args.first().and_then(|a| a.parse::<i32>().ok()).unwrap_or(9999);
    let Some(item) = server::spawn_item("phone", ahead(p) + ITEM_LIFT, Mat3::IDENTITY) else { return };

    item.set_phone_number(number);

    server::announce(&format!("Phone {number} spawned"));
}

fn gravity(args: &[&str]) {
    match args.first() {
        None => {}
        Some(&"reset") => server::set_gravity(1.0),
        Some(v) => match v.parse::<f32>() {
            Ok(g) if g.is_finite() && (-1000.0..=1000.0).contains(&g) => server::set_gravity(g / EARTH_GRAVITY),
            _ => {
                server::announce("Usage: /gravity <m/s2 | reset>");
                return;
            }
        },
    }

    server::announce(&format!("Gravity is {} m/s2", server::get_gravity() * EARTH_GRAVITY));
}

ferrosa_wit::plugin!(Sandbox);
