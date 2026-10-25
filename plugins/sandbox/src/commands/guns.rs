use ferrosa_wit::{Player, server};

const GUNS: [(&str, Option<&str>); 7] = [
    ("ak47", Some("ak47mag")),
    ("m16", Some("m16mag")),
    ("mp5", Some("mp5mag")),
    ("uzi", Some("uzimag")),
    ("pistol", Some("pistolmag")),
    ("magnum", Some("magnummag")),
    ("auto5", None)
];

pub fn spawn(p: &Player) {
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
