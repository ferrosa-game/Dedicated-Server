use ferrosa_map::file_types::{
    glb::Glb,
    it3::{It3Collision, It3Group, It3Indices},
};
use ferrosa_protocol::clientbound::game::ItemKind;
use glam::Vec3;
use serde::{Deserialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

use crate::{
    hull::ConvexHull,
    vehicle_body::{child, mesh_faces, numbered, vec3_property},
};

#[derive(Debug)]
pub struct ItemType {
    pub name: String,
    pub price: i32,
    pub mass: f32,
    pub can_collide: bool,
    pub bounds: Vec3,
    pub inv_inertia: Vec3,
    pub hull: Option<ConvexHull>,
    pub is_gun: bool,
    /// The one-handed guns: the holder aims across the body, turning the chest the other way (messedUpAiming).
    pub mirrored_aim: i32,
    pub hands: i32,
    pub magazine_ammo: i32,
    /// Which of the five pockets (inventory slots 2 to 6) the item fits in.
    pub pockets: [i32; 5],
    pub hold_pos: [Vec3; 2],
    pub hold_rot: [[f32; 4]; 2],
    pub fire_rate: i32,
    pub bullet_type: i32,
    pub bullet_velocity: f32,
    pub bullet_spread: f32,
    pub can_mount_to: [i32; ItemKind::COUNT],
    pub gun_hold_pos: Vec3,
}

impl ItemType {
    pub fn new(name: &str, price: i32, mass: f32, can_collide: bool, bounds: Vec3) -> Self {
        Self {
            name: name.to_string(),
            price,
            mass,
            can_collide,
            bounds,
            inv_inertia: inverse_inertia(bounds),
            hull: None,
            is_gun: false,
            mirrored_aim: 0,
            hands: 0,
            magazine_ammo: 0,
            pockets: [0; 5],
            hold_pos: [Vec3::ZERO; 2],
            hold_rot: [[0.0; 4]; 2],
            fire_rate: 0,
            bullet_type: 0,
            bullet_velocity: 0.0,
            bullet_spread: 0.0,
            can_mount_to: [0; ItemKind::COUNT],
            gun_hold_pos: Vec3::ZERO,
        }
    }
}

pub fn inverse_inertia(b: Vec3) -> Vec3 {
    if !(b.x > 0.0 && b.y > 0.0 && b.z > 0.0) {
        return Vec3::ONE;
    }

    let (x2, y2, z2) = (b.x * b.x, b.y * b.y, b.z * b.z);

    Vec3::new(1.0 / ((y2 + z2) / 3.0), 1.0 / ((z2 + x2) / 3.0), 1.0 / ((y2 + x2) / 3.0))
}

fn collision(glb: &Glb, root: usize) -> Result<Option<It3Collision>, String> {
    let pieces = numbered(glb, root, "COL")?;
    if pieces.is_empty() {
        return Ok(None);
    }

    let mut verts: Vec<Vec3> = Vec::new();
    let mut groups = Vec::new();

    for n in pieces {
        let (corners, faces) = mesh_faces(glb, n)?;
        let base = verts.len();
        let mean = corners.iter().fold(Vec3::ZERO, |s, p| s + p) * (1.0 / corners.len().max(1) as f32);
        let center = vec3_property(glb, n, "center")?.unwrap_or(mean);
        let own: Vec<i32> = (base..base + corners.len()).map(|i| i as i32).collect();
        let hull_verts = glb.property::<Vec<i32>>(n, "hull_verts").map_err(|e| format!("{}: hull_verts: {e}", glb.name(n)))?.unwrap_or(own);
        let faces = faces
            .into_iter()
            .map(|f| {
                let [a, b, c] = f.map(|c| (base + c) as i32);
                It3Indices { count: 4, items: vec![a, a, b, c] }
            })
            .collect::<Vec<_>>();

        verts.extend(corners);
        groups.push(It3Group { center, verts: It3Indices { count: hull_verts.len() as u32, items: hull_verts }, face_count: faces.len() as u32, faces });
    }

    Ok(Some(It3Collision { sub_version: 0, vert_count: verts.len() as u32, verts, group_count: groups.len() as u32, groups }))
}

fn property<T: DeserializeOwned>(glb: &Glb, node: usize, key: &str) -> Result<Option<T>, String> {
    glb.property(node, key).map_err(|e| format!("{}: {key}: {e}", glb.name(node)))
}

#[derive(Deserialize)]
struct Prism {
    wide: f32,
    narrow: f32,
    height: f32,
    #[serde(default)]
    melon: bool,
}

fn from_model(path: &Path, kinds: &dyn Fn(&str) -> Option<usize>) -> Result<ItemType, String> {
    let glb = Glb::load(path).map_err(|e| e.to_string())?;
    let root = glb.root().ok_or("no root object")?;

    macro_rules! p {
        ($key:literal) => {
            property(&glb, root, $key)?
        };
    }

    let bounds = child(&glb, root, "BOUNDS").map(|n| glb.scale(n)).ok_or("BOUNDS is required")?;
    let name: String = p!("display_name").ok_or("display_name is required")?;
    let mass: f32 = p!("mass").ok_or("mass is required")?;
    let mut t = ItemType::new(&name, p!("price").unwrap_or(0), mass, p!("can_collide").unwrap_or(false), bounds);

    t.is_gun = p!("is_gun").unwrap_or(false);
    t.mirrored_aim = p!("mirrored_aim").unwrap_or(0);
    t.hands = p!("hands").unwrap_or(0);
    t.magazine_ammo = p!("magazine_ammo").unwrap_or(0);
    t.pockets = p!("pockets").unwrap_or([0; 5]);
    let hold_pos: [[f32; 3]; 2] = p!("hold_pos").unwrap_or([[0.0; 3]; 2]);
    t.hold_pos = hold_pos.map(Vec3::from_array);
    t.hold_rot = p!("hold_rot").unwrap_or([[0.0; 4]; 2]);
    t.fire_rate = p!("fire_rate").unwrap_or(0);
    t.bullet_type = p!("bullet_type").unwrap_or(0);
    t.bullet_velocity = p!("bullet_velocity").unwrap_or(0.0);
    t.bullet_spread = p!("bullet_spread").unwrap_or(0.0);
    t.gun_hold_pos = Vec3::from_array(p!("gun_hold_pos").unwrap_or([0.0; 3]));

    let mounts: Vec<String> = p!("mounts_to").unwrap_or_default();
    for m in mounts {
        let k = kinds(&m).ok_or(format!("mounts_to: no item model {m}"))?;
        t.can_mount_to[k] = 1;
    }

    let inertia: Option<[f32; 3]> = p!("inertia");
    if let Some(inertia) = inertia {
        t.inv_inertia = Vec3::ONE / Vec3::from_array(inertia);
    }

    let prism: Option<Prism> = p!("prism");
    t.hull = match (collision(&glb, root)?, prism) {
        (Some(c), _) => Some(ConvexHull::from_it3(&c)),
        (None, Some(p)) => Some(ConvexHull::prism(p.wide, p.narrow, p.height, p.melon)),
        (None, None) if t.can_collide => Some(ConvexHull::bounding_box(bounds)),
        _ => None,
    };

    Ok(t)
}

/// The item models' folder.
fn models_dir() -> PathBuf {
    Path::new("data").join("models").join("items")
}

/// Each item type's model (data/models/items/<NN>_<name>.glb, `NN` its number), if it has one.
fn model_files() -> Vec<Option<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(models_dir()).map(|d| d.flatten().map(|e| e.path()).collect()).unwrap_or_default();
    files.sort();

    (0..ItemKind::COUNT)
        .map(|kind| {
            let prefix = format!("{kind:02}_");
            files.iter().find(|p| p.extension().is_some_and(|e| e == "glb") && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix))).cloned()
        })
        .collect()
}

/// The item type table, each type from its model; a type without one (or with one that is wrong) is left empty.
pub fn item_types() -> Vec<ItemType> {
    let files = model_files();
    let stems: Vec<Option<String>> = files.iter().map(|f| f.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).map(str::to_string)).collect();
    let kinds = |stem: &str| stems.iter().position(|s| s.as_deref() == Some(stem));
    let empty = || ItemType::new("", 0, 0.0, false, Vec3::ZERO);

    files
        .iter()
        .enumerate()
        .map(|(kind, file)| match file {
            Some(path) => from_model(path, &kinds).unwrap_or_else(|e| {
                eprintln!("[Items] {}: {e}", path.display());
                empty()
            }),
            None => {
                eprintln!("[Items] no model for item type {kind} in {}", models_dir().display());
                empty()
            }
        })
        .collect()
}

/// An item type's model name: its number and its name in lower case words (`09_uzi`, `39_computer`).
pub fn model_name(kind: usize, name: &str) -> String {
    let words: Vec<String> = name.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect();
    format!("{kind:02}_{}", words.join("_"))
}
