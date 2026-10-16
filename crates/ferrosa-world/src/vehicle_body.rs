use ferrosa_map::file_types::{
    glb::{Glb, Triangles, any_vec3::AnyVec3},
    sbv::VehicleBodyFile,
};
use glam::Vec3;

use crate::hull::ConvexHull;

const MAX_MESH_EDGES: usize = 256;
const MIN_NODES: usize = 4;
const COPLANAR: f32 = 0.9999;

#[derive(Clone, Debug)]
pub struct VehicleBody {
    /// The hull's corners: the chassis' mass points, which the wheels hang between.
    pub nodes: Vec<Vec3>,
    pub wheels: Vec<Vec3>,
    pub seats: Vec<Vec3>,
    /// The hull's triangles over the nodes.
    pub hull_faces: Vec<Vec<usize>>,
    /// The collision mesh over the nodes, as written (about the model's origin, which is the vehicle's position), with
    /// its edges.
    pub mesh: ConvexHull,
    pub trace_verts: Vec<Vec3>,
    pub trace_faces: Vec<Vec<usize>>,
    pub windows: Vec<Vec<[Vec3; 3]>>,
    /// The track and wheelbase when they are not the wheel mounts' (the helicopter keeps park5's without its wheels).
    pub track: Option<f32>,
    pub wheelbase: Option<f32>,
}

fn register_unique_pair(edges: &mut Vec<[usize; 2]>, a: usize, b: usize) {
    if edges.len() >= MAX_MESH_EDGES || edges.iter().any(|&[x, y]| (x == a && y == b) || (x == b && y == a)) {
        return;
    }

    edges.push([a, b]);
}

pub fn place(mut file: VehicleBodyFile) -> VehicleBodyFile {
    if let Some(offset) = file.offset {
        let shift = |p: &mut Vec3| *p -= offset;

        file.nodes.iter_mut().for_each(|n| shift(&mut n.pos));
        file.vertices.iter_mut().for_each(shift);
        file.parts.iter_mut().flat_map(|p| p.points.iter_mut()).for_each(shift);
        file.wheels.iter_mut().for_each(|w| shift(&mut w.pos));
    }

    file
}

fn hull_edges(verts: &[Vec3], faces: &[[usize; 3]]) -> Vec<[usize; 2]> {
    let normal = |f: &[usize; 3]| (verts[f[1]] - verts[f[0]]).cross(verts[f[2]] - verts[f[0]]).normalize_or_zero();
    let normals: Vec<Vec3> = faces.iter().map(normal).collect();
    let flat = |a: usize, b: usize| {
        let sharing: Vec<usize> = (0..faces.len()).filter(|&i| faces[i].contains(&a) && faces[i].contains(&b)).collect();
        matches!(sharing.as_slice(), [i, j] if normals[*i].dot(normals[*j]) > COPLANAR)
    };

    let mut edges = Vec::new();
    for f in faces {
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            if a != b && !flat(a, b) {
                register_unique_pair(&mut edges, a, b);
            }
        }
    }
    edges
}

pub fn numbered(glb: &Glb, root: usize, prefix: &str) -> Result<Vec<usize>, String> {
    let mut found: Vec<(usize, usize)> = glb
        .children(root)
        .into_iter()
        .filter_map(|n| glb.name(n).strip_prefix(prefix)?.strip_prefix('.')?.split('.').next()?.parse().ok().map(|k| (k, n)))
        .collect();
    found.sort_unstable();

    match found.iter().enumerate().find(|&(i, &(k, _))| i != k) {
        Some((i, _)) => Err(format!("{prefix}.{i:02} is missing")),
        None => Ok(found.into_iter().map(|(_, n)| n).collect()),
    }
}

pub fn child(glb: &Glb, root: usize, name: &str) -> Option<usize> {
    let matches = |n: &str| n.strip_prefix(name).is_some_and(|rest| rest.is_empty() || rest.strip_prefix('.').is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())));
    glb.children(root).into_iter().find(|&n| matches(glb.name(n)))
}

fn weld(t: Triangles) -> Triangles {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut seen: std::collections::HashMap<[u32; 3], usize> = std::collections::HashMap::new();
    let index: Vec<usize> = t
        .positions
        .iter()
        .map(|p| {
            *seen.entry(p.to_array().map(f32::to_bits)).or_insert_with(|| {
                positions.push(*p);
                positions.len() - 1
            })
        })
        .collect();

    Triangles { positions, triangles: t.triangles.iter().map(|tri| tri.map(|c| index[c])).collect() }
}

pub fn mesh_faces(glb: &Glb, node: usize) -> Result<(Vec<Vec3>, Vec<[usize; 3]>), String> {
    let name = glb.name(node).to_string();
    let t = weld(glb.triangles(node).map_err(|e| format!("{name}: {e}"))?.ok_or(format!("{name} has no mesh"))?);

    Ok((t.positions, t.triangles.iter().map(|&[a, b, c]| [a, c, b]).collect()))
}

/// What the client does not draw: the physics objects, by their names' prefixes.
pub const PHYSICS_PREFIXES: [&str; 6] = ["COL", "TRACE_", "wheel.", "window.", "seat.", "BOUNDS"];

fn visible_faces(glb: &Glb, root: usize) -> Result<(Vec<Vec3>, Vec<[usize; 3]>), String> {
    let physics = |name: &str| PHYSICS_PREFIXES.iter().any(|p| name.starts_with(p));
    let glass = |name: &str| name.to_ascii_lowercase().contains("glass");
    let t = weld(glb.triangles_under(root, &physics, &glass).map_err(|e| format!("the visible model: {e}"))?);

    Ok((t.positions, t.triangles.iter().map(|&[a, b, c]| [a, c, b]).collect()))
}

pub fn vec3_property(glb: &Glb, node: usize, key: &str) -> Result<Option<Vec3>, String> {
    glb.property::<AnyVec3>(node, key).map(|v| v.map(|v| v.0)).map_err(|e| format!("{}: {key}: {e}", glb.name(node)))
}

impl VehicleBody {
    pub fn read(glb: &Glb, root: usize) -> Result<Self, String> {
        let prop = |key: &str| glb.property::<f32>(root, key).map_err(|e| format!("{key}: {e}"));

        let hull = child(glb, root, "COL_hull").ok_or("COL_hull is required")?;
        let (nodes, faces) = mesh_faces(glb, hull)?;
        if nodes.len() < MIN_NODES {
            return Err(format!("COL_hull has {} corners, at least {MIN_NODES} needed", nodes.len()));
        }
        let faces: Vec<[usize; 3]> = faces.into_iter().filter(|f| f[0] != f[1] && f[1] != f[2] && f[2] != f[0]).collect();

        let edges = hull_edges(&nodes, &faces);
        let corners = faces.iter().map(|&[a, b, c]| [a, b, c, c]).collect();
        let mesh = ConvexHull::new(nodes.clone(), corners, edges);

        let wheels = numbered(glb, root, "wheel")?.into_iter().map(|n| glb.translation(n)).collect();
        let seats = numbered(glb, root, "seat")?.into_iter().map(|n| glb.translation(n)).collect();

        let (trace_verts, trace_faces) = match child(glb, root, "TRACE_body") {
            Some(n) => mesh_faces(glb, n)?,
            None => visible_faces(glb, root)?,
        };
        let trace_faces = trace_faces.iter().map(|f| f.to_vec()).collect();

        let windows = numbered(glb, root, "window")?
            .into_iter()
            .map(|n| mesh_faces(glb, n).map(|(verts, faces)| faces.iter().map(|f| f.map(|c| verts[c])).collect()))
            .collect::<Result<Vec<_>, String>>()?;

        Ok(Self {
            nodes,
            wheels,
            seats,
            hull_faces: faces.iter().map(|f| f.to_vec()).collect(),
            mesh,
            trace_verts,
            trace_faces,
            windows,
            track: prop("track")?,
            wheelbase: prop("wheelbase")?,
        })
    }
}
