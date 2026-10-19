use ferrosa_physics::RotMatrix;
use glam::{IVec3, Vec3};

use crate::{
    area::{AreaGrid, CUBE, FOOTPRINT, MESH, TYPE_MASK, cell_index},
    city_objects::{object_pose, sphere_intersect_object},
    collide::{Best, calculate_face_normal, cell_point, footprint_origin, open_faces, segment_intersect_face, wall_skipped},
    item_sets::entry_pose,
    meshes::BlockMeshes,
    trace::{AreaFrame, cvtt},
};




/// sphere_project_onto_triangle_face: a centre in front of the face within `radius` touches it where it drops onto the
/// face along the normal. Returns the point and its distance.
pub fn sphere_project_onto_triangle(center: Vec3, n: Vec3, a: Vec3, b: Vec3, c: Vec3, radius: f32) -> Option<(Vec3, f32)> {
    let d = (center - a).dot(n);

    if d > radius || 0.0 > d {
        return None;
    }

    let r = -radius;
    let end = n * r + center;
    let (_, p) = segment_intersect_face(n, center, end, a, b, c)?;
    let (x, y, z) = (p.x - center.x, p.y - center.y, p.z - center.z);

    Some((p, (z * z + (x * x + y * y)).sqrt()))
}

/// sphere_intersect_segment_interior: the point of an edge nearest the centre, if it lies within the edge and within
/// `radius`. Returns the point, the normal from it to the centre and the distance.
pub fn sphere_intersect_segment(center: Vec3, p0: Vec3, p1: Vec3, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let e = p1 - p0;
    let len = e.length();
    let v = center - p0;

    let u = if len != 0.0 {
        let inv = 1.0 / len;
        e * inv
    } else {
        Vec3::ZERO
    };

    let t = v.dot(u);

    if t <= 0.0 || len <= t {
        return None;
    }

    let q = u * t + p0;
    let d = center - q;
    let dist = d.length();

    if radius <= dist {
        return None;
    }

    let n = if dist != 0.0 {
        let inv = 1.0 / dist;
        d * inv
    } else {
        Vec3::ZERO
    };

    Some((q, n, dist))
}

/// A face the centre is in front of: its two triangles, then its four edges.
fn quad(best: &mut Best, center: Vec3, n: Vec3, [a, b, c, d]: [Vec3; 4], radius: f32) {
    if (center - a).dot(n) <= 0.0 {
        return;
    }

    best.take(sphere_project_onto_triangle(center, n, a, b, c, radius).map(|(p, d)| (p, n, d)));
    best.take(sphere_project_onto_triangle(center, n, a, c, d, radius).map(|(p, d)| (p, n, d)));

    for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
        best.take(sphere_intersect_segment(center, p, q, radius));
    }
}

/// sphere_intersect_voxel_cube_faces: the cube's open faces (those the cell word marks).
fn cube_faces(center: Vec3, cell: IVec3, s: f32, v: u32, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let mut best = Best::default();

    for (n, corners) in open_faces(cell, s, v) {
        quad(&mut best, center, n, corners, radius);
    }

    best.hit()
}

/// sphere_intersect_voxel_custom_shape: a custom block's faces, then its walls from whichever side the centre is on.
fn custom_shape(meshes: &BlockMeshes, center: Vec3, cell: IVec3, s: f32, v: u32, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let mesh = meshes.get(v & 65535)?;
    let sc = cell_point(cell, s);
    let mut best = Best::default();

    for q in &mesh.quads {
        let [a, b, c, d] = q.map(|i| sc(mesh.verts[i as usize]));
        quad(&mut best, center, calculate_face_normal(a, b, c), [a, b, c, d], radius);
    }

    for (i, w) in mesh.walls.iter().enumerate() {
        if wall_skipped(v, i) {
            continue;
        }

        let [a, b, c, d] = w.map(sc);
        let n = calculate_face_normal(a, b, c);

        if (center - a).dot(n) > 0.0 {
            quad(&mut best, center, n, [a, b, c, d], radius);
        } else {
            quad(&mut best, center, calculate_face_normal(d, c, b), [d, c, b, a], radius);
        }
    }

    best.hit()
}

/// sphere_intersect_item_type_mesh: a sphere at rest against each face of an item's mesh placed at `pos` turned by `rot`
/// (both triangles of a face under the first one's normal).
fn sphere_intersect_item(hull: &crate::hull::ConvexHull, pos: Vec3, rot: &RotMatrix, center: Vec3, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let world = hull.world_verts(pos, rot);
    let mut best = Best::default();

    for f in hull.faces() {
        let [a, b, c, d] = f.verts.map(|i| world[i]);
        let n = calculate_face_normal(a, b, c);

        best.take(sphere_project_onto_triangle(center, n, a, b, c, radius).map(|(p, dist)| (p, n, dist)));
        best.take(sphere_project_onto_triangle(center, n, a, c, d, radius).map(|(p, dist)| (p, n, dist)));
    }

    best.hit()
}

/// collision_cast_area: a sphere at rest against the cells of the level area it overlaps. An item of a cell's item set
/// or an area object it touches ends the search at once, its contact left in the area's frame as the binary leaves it.
pub fn collision_cast_area(area: &AreaGrid, meshes: &BlockMeshes, center: Vec3, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let f = AreaFrame::new(area);
    let (c3, mn, mx) = (center.to_array(), f.min.to_array(), f.max.to_array());

    if (0..3).any(|k| mn[k] > c3[k] + radius) || (0..3).any(|k| c3[k] - radius > mx[k]) {
        return None;
    }

    let lc = f.to_local(center);
    let la = lc.to_array();
    let lo = la.map(|v| cvtt((v - radius) * f.inv));
    let hi = la.map(|v| cvtt((v + radius) * f.inv));
    let mut best = Best::default();

    for y in lo[1]..=hi[1] {
        for z in lo[2]..=hi[2] {
            for x in lo[0]..=hi[0] {
                let Some(rec) = area.record(x, y, z) else { continue };
                let cell = IVec3::new(x, y, z);
                let idx = cell_index(x, y, z);
                let word = rec.item_set[idx];

                if word != 0
                    && let Some(set) = area.item_sets.set(word)
                {
                    for e in &set.entries {
                        let Some(hull) = area.item_sets.hull(e.kind) else { continue };
                        let (pos, rot) = entry_pose(e, cell, f.size, word >> 24 & 3);

                        if let Some(hit) = sphere_intersect_item(hull, pos, &rot, lc, radius) {
                            return Some(hit);
                        }
                    }
                }

                let object = rec.object[idx];

                if object as i32 > 0 {
                    let (kind, pos, rot) = object_pose(object, cell, f.size);

                    if let Some(hit) = sphere_intersect_object(kind, pos, &rot, lc, radius) {
                        return Some(hit);
                    }
                }

                for layer in 0..2 {
                    let v = rec.layer(layer)[idx];

                    let hit = match v & TYPE_MASK {
                        CUBE => cube_faces(lc, cell, f.size, v, radius),
                        MESH => custom_shape(meshes, lc, cell, f.size, v, radius),
                        FOOTPRINT => {
                            let o = footprint_origin(cell, v);
                            let w = area.layer(layer, o);
                            custom_shape(meshes, lc, o, f.size, w, radius)
                        }
                        _ => None,
                    };

                    best.take(hit.map(|(p, n, d)| {
                        let p = f.rotate_back(p);
                        (p + f.origin, f.rotate_back(n), d)
                    }));
                }
            }
        }
    }

    best.hit()
}

/// sphere_cast_level: a sphere at rest against the level's areas (not the terrain). Returns the contact point, normal and
/// distance from the centre.
pub fn sphere_cast_level(area: &AreaGrid, meshes: &BlockMeshes, center: Vec3, radius: f32) -> Option<(Vec3, Vec3, f32)> {
    let mut best = Best::default();

    best.take(collision_cast_area(area, meshes, center, radius));
    best.hit()
}
