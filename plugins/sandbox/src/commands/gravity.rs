use ferrosa_wit::server;

const EARTH_GRAVITY: f32 = 9.8;

pub fn set(args: &[&str]) {
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
