use glam::Vec3;

use ferrosa_protocol::clientbound::game::VehicleKind;
use rayon::prelude::*;

use ferrosa_map::file_types::glb::Glb;
use serde::de::DeserializeOwned;

use crate::vehicle_body::VehicleBody;

pub const VEHICLE_TYPES: usize = 17;
const NODE_SEARCH: f32 = 65536.0;
const WHEEL_SEARCH_SCALE: f32 = 4.0;
const LOWER_NODE_MASS: f32 = 4.0;
const UPPER_NODE_MASS: f32 = 1.0;
const DRIVEN_WHEELS: [usize; 2] = [2, 3];
const REV_LIMIT_DEGREES: f32 = 600.0;
const MAX_RPM: f64 = 7000.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct ChassisNode {
    pub pos: Vec3,
    pub mass: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WheelDef {
    pub state: i32,
    pub mount: Vec3,
    pub mass: f32,
    pub radius: f32,
    /// Its mount, where it rests in the model.
    pub local_pos: Vec3,
    pub spring: f32,
    pub damping: f32,
    pub travel_damping: f32,
    pub drive: i32,
    /// Its mount around the centre of mass.
    pub mass_offset: Vec3,
}

/// A vehicle type.
#[derive(Clone, Debug, Default)]
pub struct VehicleType {
    /// The state a vehicle of this type spawns in (controllableState).
    pub controllable_state: i32,
    pub name: String,
    pub price: i32,
    pub mass: f32,
    /// The chassis' mass points: the collision hull's corners, heavier below the floor (the model's `floor` height).
    pub nodes: Vec<ChassisNode>,
    /// Where the vehicle's position is in its model: the model's origin, so zero (traffic places cars by it).
    pub centroid: Vec3,
    /// The engine's power as a multiple of 213.
    pub engine_power: f32,
    pub wheels: Vec<WheelDef>,
    pub track: f32,
    pub wheelbase: f32,
    pub seats: Vec<Vec3>,
    pub center_of_mass: Vec3,
    /// The model's origin (the vehicle's position) around the centre of mass.
    pub centroid_offset: Vec3,
    pub inertia: Vec3,
    /// The trace mesh's vertices, which the vehicle's bounds cover.
    pub render_verts: Vec<Vec3>,
    pub mesh: Option<crate::hull::ConvexHull>,
    pub chassis_faces: Vec<Vec<usize>>,
    pub render_faces: Vec<Vec<usize>>,
    pub windows: Vec<Vec<[Vec3; 3]>>,
    pub driven_wheels: Vec<usize>,
    pub rev_limit: f32,
    pub max_rpm: f64,
    pub torque_curve: TorqueCurve,
    pub rotor: bool,
    pub train: bool,
}

/// The engine's torque over its speed: the cars', the van's or the minivan's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TorqueCurve {
    #[default]
    Car,
    Van,
    Minivan,
}

/// The settings every wheel of a type shares (the model's `wheel_*` and `drive` properties).
#[derive(Clone, Copy)]
struct WheelSpec {
    radius: f32,
    mass: f32,
    spring: f32,
    damping: f32,
    travel_damping: f32,
    drive: i32,
}

fn prop<T: DeserializeOwned>(glb: &Glb, root: usize, key: &str) -> Result<Option<T>, String> {
    glb.property(root, key).map_err(|e| format!("{key}: {e}"))
}

impl VehicleType {
    /// A wheel resting at each mount; then the track and wheelbase (as given, or between the mounts).
    fn place_wheels(&mut self, body: &VehicleBody, spec: &WheelSpec) {
        for &mount in &body.wheels {
            self.wheels.push(WheelDef {
                state: 0,
                mount,
                mass: spec.mass,
                radius: spec.radius,
                local_pos: mount,
                spring: spec.spring,
                damping: spec.damping,
                travel_damping: spec.travel_damping,
                drive: spec.drive,
                ..Default::default()
            });
        }

        let wp = |k: usize| body.wheels.get(k).copied().unwrap_or(Vec3::ZERO);

        self.track = body.track.unwrap_or_else(|| (wp(0).x - wp(1).x).abs());
        self.wheelbase = body.wheelbase.unwrap_or_else(|| (wp(0).z - wp(2).z).abs());
    }

    /// vehicletype_attach_wheels_sized's centre of mass, inertia and the wheels around the centre of mass.
    fn mass_properties(&mut self) {
        let mut com = Vec3::ZERO;
        let mut total = 0.0f32;

        for n in &self.nodes {
            let m = n.mass;
            com = n.pos * m + com;
            total += m;
        }

        let inv = 1.0 / total;
        com *= inv;
        self.center_of_mass = com;

        let (inertia, total) = self.nodes.iter().fold((Vec3::ZERO, 0.0f32), |(acc, total), n| {
            let (dy, dx, dz) = ((n.pos.y - com.y) * (n.pos.y - com.y), (n.pos.x - com.x) * (n.pos.x - com.x), (n.pos.z - com.z) * (n.pos.z - com.z));
            let m = n.mass;
            (Vec3::new((dy + dz) * m + acc.x, (dz + dx) * m + acc.y, (dy + dx) * m + acc.z), total + m)
        });

        let inv = 1.0 / total;
        self.inertia = inertia * inv;
        self.centroid_offset = -com;

        for w in self.wheels.iter_mut() {
            w.mass_offset = w.mount - com;
        }
    }

    /// vehicletype_attach_wheels' inertia about the origin, and each wheel at its mount without a centre of mass.
    fn mass_properties_unsized(&mut self, body: &VehicleBody) {
        let (inertia, total) = self.nodes.iter().fold((Vec3::ZERO, 0.0f32), |(acc, total), n| {
            let (dy, dx, dz) = (n.pos.y * n.pos.y, n.pos.x * n.pos.x, n.pos.z * n.pos.z);
            let m = n.mass;
            (Vec3::new((dy + dz) * m + acc.x, (dz + dx) * m + acc.y, (dy + dx) * m + acc.z), total + m)
        });

        let inv = 1.0 / total;
        self.inertia = inertia * inv;

        for (w, mount) in self.wheels.iter_mut().zip(&body.wheels) {
            w.local_pos = *mount;
            w.mass_offset = w.local_pos;
        }
    }

    /// vehicletype_precompute_physics: the node masses as shares. The game also moved everything about the nodes'
    /// mean here; models are written about the point the game moved them to, so nothing moves.
    fn precompute_physics(&mut self) {
        let total = self.nodes.iter().fold(0.0f32, |t, n| t + n.mass);
        for n in self.nodes.iter_mut() {
            n.mass /= total;
        }
    }

    /// A vehicle model (models/vehicles/<name>.glb) as the type its properties and physics objects make.
    pub fn from_model(path: &std::path::Path) -> Result<Self, String> {
        let glb = Glb::load(path).map_err(|e| e.to_string())?;
        let root = glb.root().ok_or("no root object")?;
        let train = prop(&glb, root, "train")?.unwrap_or(false);
        let body = VehicleBody::read(&glb, root)?;

        let required = |key: &str| prop::<f32>(&glb, root, key)?.ok_or(format!("{key} is required"));
        let wheel_required = |key: &str| if body.wheels.is_empty() { Ok(0.0) } else { required(key) };

        let spec = WheelSpec {
            radius: prop(&glb, root, "wheel_radius")?.map_or_else(|| wheel_required("wheel_radius"), Ok)?,
            mass: prop(&glb, root, "wheel_mass")?.map_or_else(|| wheel_required("wheel_mass"), Ok)?,
            spring: prop(&glb, root, "wheel_spring")?.unwrap_or(0.25),
            damping: prop(&glb, root, "wheel_damping")?.unwrap_or(0.5),
            travel_damping: prop(&glb, root, "wheel_travel_damping")?.unwrap_or(0.625),
            drive: prop(&glb, root, "drive")?.unwrap_or(0),
        };

        let driven_wheels: Vec<usize> = prop(&glb, root, "driven_wheels")?.unwrap_or(DRIVEN_WHEELS.to_vec());
        if !body.wheels.is_empty()
            && let Some(w) = driven_wheels.iter().find(|&&w| w >= body.wheels.len())
        {
            return Err(format!("driven wheel {w} is not a wheel"));
        }

        let mut t = VehicleType {
            controllable_state: prop(&glb, root, "controllable_state")?.unwrap_or(0),
            name: prop(&glb, root, "display_name")?.ok_or("display_name is required")?,
            price: prop(&glb, root, "price")?.unwrap_or(0),
            mass: required("mass")?,
            engine_power: prop(&glb, root, "engine_power")?.unwrap_or(0.0),
            driven_wheels,
            rev_limit: prop::<f32>(&glb, root, "rev_limit")?.unwrap_or(REV_LIMIT_DEGREES).to_radians(),
            max_rpm: prop(&glb, root, "max_rpm")?.unwrap_or(MAX_RPM),
            torque_curve: match prop::<String>(&glb, root, "torque_curve")?.as_deref() {
                None | Some("car") => TorqueCurve::Car,
                Some("van") => TorqueCurve::Van,
                Some("minivan") => TorqueCurve::Minivan,
                Some(other) => return Err(format!("torque_curve {other:?} is not car, van or minivan")),
            },
            rotor: prop(&glb, root, "rotor")?.unwrap_or(false),
            train,
            ..Default::default()
        };

        let floor: f32 = prop(&glb, root, "floor")?.unwrap_or(0.0);
        t.nodes = body.nodes.iter().map(|&pos| ChassisNode { pos, mass: if floor > pos.y { LOWER_NODE_MASS } else { UPPER_NODE_MASS } }).collect();
        t.place_wheels(&body, &spec);

        if train {
            t.mass_properties_unsized(&body);
        } else {
            t.mass_properties();
        }

        t.seats = body.seats.clone();
        t.render_verts = body.trace_verts.clone();
        t.mesh = Some(body.mesh.clone());
        t.chassis_faces = body.hull_faces.clone();
        t.render_faces = body.trace_faces.clone();
        t.windows = body.windows.clone();

        if !train {
            t.precompute_physics();
        }

        Ok(t)
    }
}

/// vehicletype_attach_wheels_sized's placing of a wheel, which the game hung between two chassis nodes: the node
/// nearest `p` (heights counting four times), then the nearest beyond it (heights and sideways distances counting
/// four times), and the point between them as far along as `p` is. Models place wheels at their mounts; the model
/// converter writes the game's cars' mounts at this point.
pub fn hang_point(nodes: &[Vec3], p: Vec3) -> Vec3 {
    let near = |pick: &dyn Fn(usize, Vec3) -> Option<f32>| {
        nodes.iter().enumerate().fold((0usize, NODE_SEARCH), |(best_i, best), (i, &n)| match pick(i, n) {
            Some(d) => (if best > d { i } else { best_i }, if d < best { d } else { best }),
            None => (best_i, best),
        })
    };

    let (a, _) = near(&|_, n| {
        let (dx, dy, dz) = (p.x - n.x, (p.y - n.y) * WHEEL_SEARCH_SCALE, p.z - n.z);
        Some((dz * dz + (dx * dx + dy * dy)).sqrt())
    });

    let na = nodes[a];
    let (b, _) = near(&|i, n| {
        if i == a {
            return None;
        }

        let (dx, dy, dz) = ((p.x - n.x) * WHEEL_SEARCH_SCALE, (p.y - n.y) * WHEEL_SEARCH_SCALE, p.z - n.z);
        let d = (dz * dz + (dx * dx + dy * dy)).sqrt();
        let (wa, an) = ((p - na), (n - na));
        let dot = (p.z - na.z) * an.z + ((p.y - na.y) * an.y + wa.x * an.x);

        (dot >= 0.0).then_some(d)
    });

    let nb = nodes[b];
    let to_b = (nb - p).length();
    let to_a = (na - p).length();
    let sum = to_a + to_b;

    na.lerp(nb, to_a / sum)
}

pub fn vehicle_types(data: &std::path::Path) -> Vec<VehicleType> {
    (0..VEHICLE_TYPES)
        .into_par_iter()
        .map(|k| {
            let empty =
                || VehicleType { driven_wheels: DRIVEN_WHEELS.to_vec(), rev_limit: REV_LIMIT_DEGREES.to_radians(), max_rpm: MAX_RPM, ..Default::default() };
            let Ok(kind) = VehicleKind::try_from(k as u8) else { return empty() };
            let path = model_path(data, kind);
            if !path.exists() {
                return empty();
            }

            VehicleType::from_model(&path).unwrap_or_else(|e| {
                eprintln!("[Vehicles] {}: {e}", path.display());
                empty()
            })
        })
        .collect()
}

/// A vehicle kind's model name: its kind in lower case (towncar, van2, helicopter, ...).
pub fn model_name(kind: VehicleKind) -> String {
    format!("{kind:?}").to_lowercase()
}

/// A vehicle kind's model (data/models/vehicles/<name>.glb), which defines its type.
pub fn model_path(data: &std::path::Path, kind: VehicleKind) -> std::path::PathBuf {
    data.join("models").join("vehicles").join(format!("{}.glb", model_name(kind)))
}
