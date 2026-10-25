use ferrosa_wit::{Mat3, Player, Vec3, server};

pub fn spawn(p: &Player, args: &[&str]) {
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
