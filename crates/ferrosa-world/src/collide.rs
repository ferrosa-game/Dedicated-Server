use glam::{IVec3, Vec3};

use crate::mesh::{CUBE_CORNERS, CUBE_FACES, CUBE_NORMALS};

/// The distance a search starts from: anything nearer is a hit.
pub const NO_HIT: f32 = 65536.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceHit {
    pub fraction: f32,
    pub pos: Vec3,
    pub normal: Vec3,
}

pub fn calculate_face_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let e1 = b - a;
    let e2 = c - a;

    let n = e2.cross(e1);

    let len = n.length();

    if len == 0.0 {
        return Vec3::ZERO;
    }

    let inv = 1.0 / len;

    n * inv
}

#[inline]
fn dot_sub(p: Vec3, q: Vec3, n: Vec3) -> f32 {
    (p.x - q.x) * n.x + (p.y - q.y) * n.y + (p.z - q.z) * n.z
}

pub fn segment_intersect_plane_one_sided(n: Vec3, p: Vec3, start: Vec3, end: Vec3) -> Option<(f32, Vec3)> {
    if dot_sub(p, end, n) < 0.0 {
        return None;
    }

    let d_start = dot_sub(p, start, n);

    if d_start > 0.0 {
        return None;
    }

    let d = end - start;
    let denom = n.dot(d);

    if denom == 0.0 {
        return None;
    }

    let t = d_start / denom;

    Some((t, d * t + start))
}

#[inline]
fn edge(p: Vec3, v0: Vec3, v1: Vec3, n: Vec3) -> f32 {
    let e = v1 - v0;
    let (px, py, pz) = (p.x - v0.x, p.y - v0.y, p.z - v0.z);

    let cx = e.z * py - e.y * pz;
    let cy = pz * e.x - e.z * px;
    let cz = e.y * px - e.x * py;

    cx * n.x + cy * n.y + cz * n.z
}

pub fn point_in_triangle_oriented(p: Vec3, n: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
    edge(p, a, b, n) >= 0.0 && edge(p, b, c, n) >= 0.0 && edge(p, c, a, n) >= 0.0
}

pub fn segment_intersect_face(n: Vec3, start: Vec3, end: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, Vec3)> {
    let (t, p) = segment_intersect_plane_one_sided(n, a, start, end)?;

    point_in_triangle_oriented(p, n, a, b, c).then_some((t, p))
}

/// The nearest contact so far: point, normal and distance.
#[derive(Clone, Copy, Debug)]
pub struct Best {
    pub pos: Vec3,
    pub normal: Vec3,
    pub dist: f32,
}

impl Default for Best {
    fn default() -> Self {
        Self { pos: Vec3::ZERO, normal: Vec3::ZERO, dist: NO_HIT }
    }
}

impl Best {
    /// Keeps `hit` when it is nearer than the best so far, saying whether it was.
    pub fn take(&mut self, hit: Option<(Vec3, Vec3, f32)>) -> bool {
        match hit {
            Some((pos, normal, dist)) if self.dist > dist => {
                *self = Self { pos, normal, dist };
                true
            }
            _ => false,
        }
    }

    pub fn hit(&self) -> Option<(Vec3, Vec3, f32)> {
        (NO_HIT > self.dist).then_some((self.pos, self.normal, self.dist))
    }
}

/// A point of a cell's block-mesh space (0..1 per axis) in its area's frame, cells `s` across.
pub fn cell_point(cell: IVec3, s: f32) -> impl Fn(Vec3) -> Vec3 + Copy {
    let base = cell.as_vec3() * s;

    move |p| p * s + base
}

/// The faces of a cube cell its word marks open: each one's outward normal and corners.
pub fn open_faces(cell: IVec3, s: f32, v: u32) -> impl Iterator<Item = (Vec3, [Vec3; 4])> {
    let point = cell_point(cell, s);

    (0..6).filter(move |&r| v & (1 << r) != 0 && v & (64 << r) != 0).map(move |r| (CUBE_NORMALS[r], CUBE_FACES[r].map(|i| point(CUBE_CORNERS[i]))))
}

/// Whether a custom shape's cell word turns off its wall `i`.
pub fn wall_skipped(v: u32, i: usize) -> bool {
    (v >> 16) & 1023 & (1 << (i & 31)) != 0
}

/// The origin cell a footprint cell word points back to.
pub fn footprint_origin(cell: IVec3, v: u32) -> IVec3 {
    cell - IVec3::new((v & 255) as i32, ((v >> 16) & 255) as i32, ((v >> 8) & 255) as i32)
}
