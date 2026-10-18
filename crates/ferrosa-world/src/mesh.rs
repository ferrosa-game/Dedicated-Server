use glam::{IVec3, Vec2, Vec3};

use crate::collide::calculate_face_normal;

pub const FACE_NEG_X: u32 = 1 << 16;
pub const FACE_POS_X: u32 = 2 << 16;
pub const FACE_DOWN: u32 = 3 << 16;
pub const FACE_UP: u32 = 4 << 16;
pub const FACE_NEG_Z: u32 = 5 << 16;
pub const FACE_POS_Z: u32 = 6 << 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Degree {
    Linear = 1,
    Quadratic = 2,
    Cubic = 3,
}

impl Degree {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            1 => Some(Self::Linear),
            2 => Some(Self::Quadratic),
            3 => Some(Self::Cubic),
            _ => None,
        }
    }
}

/// bezier_evaluate_scalar: the curve's slope at `t`.
pub fn bezier_derivative(deg: Option<Degree>, t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    match deg {
        Some(Degree::Linear) => p3 - p0,
        Some(Degree::Quadratic) => (t + t) * ((p3 + p0) - (p1 + p1)) - (p0 + p0) + (p1 + p1),
        Some(Degree::Cubic) => {
            let (p1x3, p2x3, p0x3) = (p1 * 3.0, p2 * 3.0, p0 * 3.0);
            let c = (p3 + ((p1x3 - p0) - p2x3)) * (3.0 * t * t);
            (((p0x3 - p1 * 6.0) + p2x3) * (t + t) + c - p0x3) + p1x3
        }
        None => 0.0,
    }
}

pub fn bezier(deg: Option<Degree>, t: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    let om = 1.0 - t;
    match deg {
        Some(Degree::Linear) => p0 * om + t * p3,
        Some(Degree::Quadratic) => (((p1 + p1) * om * t) + om * (p0 * om)) + t * (p3 * t),
        Some(Degree::Cubic) => ((p0 * om * om * om + (p1 * 3.0) * om * om * t) + (om * (p2 * 3.0)) * t * t) + t * ((p3 * t) * t),
        None => 0.0,
    }
}

#[derive(Clone, Default, Debug)]
pub struct BlockMesh {
    pub verts: Vec<Vec3>,
    pub quads: Vec<[i32; 4]>,
    pub walls: Vec<[Vec3; 4]>,
    /// Per vertex, what the client draws with: the normal, the tangent and the texture coordinate.
    pub normals: Vec<Vec3>,
    pub tangents: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    /// Per quad, its texture code (the low 12 bits of its flags), the extra draw flags (0x80 for a cell face, 1 for
    /// one facing up) and which side of the block it lies flat on, if any.
    pub face_codes: Vec<u32>,
    pub face_extra: Vec<u32>,
    pub face_side: Vec<i32>,
    /// How many quads lie flat on a side (they come first), and per side the quads that don't (bit per quad).
    pub side_faces: usize,
    pub side_masks: [u32; 6],
}

#[derive(Default)]
pub struct MeshBuilder {
    pub ctrl: [Vec3; 16],
    /// The texture coordinates at the control points (blockFaceUvScratch), kept between faces as the client does.
    pub uv_ctrl: [Vec2; 16],
    pub pos: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub tangents: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub quads: Vec<[i32; 4]>,
    pub flags: Vec<u32>,
    pub extra: Vec<u32>,
    pub walls: Vec<[Vec3; 4]>,
}

impl MeshBuilder {
    pub fn reset(&mut self) {
        self.pos.clear();
        self.normals.clear();
        self.tangents.clear();
        self.uvs.clear();
        self.quads.clear();
        self.flags.clear();
        self.extra.clear();
        self.walls.clear();
    }

    /// The texture coordinates of the four corner control points (p0, p3, p15, p12).
    pub fn set_uv_corners(&mut self, p0: [f32; 2], p3: [f32; 2], p15: [f32; 2], p12: [f32; 2]) {
        self.uv_ctrl[0] = Vec2::from(p0);
        self.uv_ctrl[3] = Vec2::from(p3);
        self.uv_ctrl[15] = Vec2::from(p15);
        self.uv_ctrl[12] = Vec2::from(p12);
    }

    fn push_vertex(&mut self, p: Vec3, n: Vec3, t: Vec3, uv: Vec2) {
        self.pos.push(p);
        self.normals.push(n);
        self.tangents.push(t);
        self.uvs.push(uv);
    }

    fn dup_vertex(&mut self, i: i32) -> i32 {
        let i = i as usize;
        self.push_vertex(self.pos[i], self.normals[i], self.tangents[i], self.uvs[i]);
        self.pos.len() as i32 - 1
    }

    pub fn set_corners(&mut self, p0: [f32; 3], p3: [f32; 3], p15: [f32; 3], p12: [f32; 3]) {
        self.ctrl[0] = Vec3::from(p0);
        self.ctrl[3] = Vec3::from(p3);
        self.ctrl[15] = Vec3::from(p15);
        self.ctrl[12] = Vec3::from(p12);
    }

    /// append_bezier_block_face: a `u_seg` by `v_seg` grid over the bezier patch of the control points, each vertex
    /// with its texture coordinate (linear across `uv_ctrl`), its normal (dP/dv x dP/du) and its tangent (dP/du), and
    /// a quad per square, vertices shared only along the first row and column.
    pub fn append_bezier(&mut self, u_seg: i32, u_deg: Option<Degree>, v_seg: i32, v_deg: Option<Degree>, face_flags: u32) {
        let first = self.pos.len() as i32;
        let (du_step, dv_step) = (1.0 / u_seg as f32, 1.0 / v_seg as f32);
        let mut sv = 0.0f32;
        for _ in 0..=v_seg {
            let col = |c: usize, f: fn(Option<Degree>, f32, f32, f32, f32, f32) -> f32| {
                let e = |k: usize| {
                    let p = |r: usize| self.ctrl[r * 4 + c][k];
                    f(v_deg, sv, p(0), p(1), p(2), p(3))
                };
                Vec3::new(e(0), e(1), e(2))
            };

            let row: [Vec3; 4] = std::array::from_fn(|c| col(c, bezier));
            let row_dv: [Vec3; 4] = std::array::from_fn(|c| col(c, bezier_derivative));
            let row_uv: [Vec2; 4] = std::array::from_fn(|c| {
                let (a, d) = (self.uv_ctrl[c], self.uv_ctrl[12 + c]);
                Vec2::new(a.x * (1.0 - sv) + sv * d.x, a.y * (1.0 - sv) + sv * d.y)
            });

            let mut su = 0.0f32;

            for _ in 0..=u_seg {
                let along = |r: &[Vec3; 4], f: fn(Option<Degree>, f32, f32, f32, f32, f32) -> f32| {
                    let e = |k: usize| f(u_deg, su, r[0][k], r[1][k], r[2][k], r[3][k]);
                    Vec3::new(e(0), e(1), e(2))
                };

                let p = along(&row, bezier);
                let uv = Vec2::new(row_uv[0].x * (1.0 - su) + su * row_uv[3].x, row_uv[0].y * (1.0 - su) + su * row_uv[3].y);
                let du = along(&row, bezier_derivative);
                let dv = along(&row_dv, bezier);
                let n = Vec3::new(dv.y * du.z - dv.z * du.y, dv.z * du.x - du.z * dv.x, du.y * dv.x - du.x * dv.y);
                let len = (n.z * n.z + (n.y * n.y + n.x * n.x)).sqrt();

                let n = if len != 0.0 {
                    let k = 1.0 / len;
                    Vec3::new(k * n.x, k * n.y, k * n.z)
                } else {
                    Vec3::ZERO
                };

                let tl = (du.y * du.y + du.x * du.x + du.z * du.z).sqrt();

                let t = if tl != 0.0 {
                    let k = 1.0 / tl;
                    Vec3::new(du.x * k, du.y * k, k * du.z)
                } else {
                    Vec3::ZERO
                };

                self.push_vertex(p, n, t, uv);

                su += du_step;
            }

            sv += dv_step;
        }

        let mut top = first;
        let mut bottom = top + u_seg + 1;

        for r in 0..v_seg {
            for q in 0..u_seg {
                let (a, b) = if q > 0 || r > 0 {
                    let a = self.dup_vertex(top + q);
                    (a, if r == 0 { top + q + 1 } else { self.dup_vertex(top + q + 1) })
                } else {
                    (top + q, top + q + 1)
                };

                let d = if q > 0 { self.dup_vertex(bottom + q) } else { bottom + q };

                self.quads.push([a, b, bottom + 1 + q, d]);
                self.flags.push(face_flags);
                self.extra.push(0);
            }

            top += u_seg + 1;
            bottom += u_seg + 1;
        }
    }

    pub fn add_box(&mut self, p: &[Vec3; 8], tex: u32, mask: u32) {
        for (r, f) in CUBE_FACES.iter().enumerate() {
            if mask & (1 << r) == 0 {
                continue;
            }

            let n = calculate_face_normal(p[f[0]], p[f[1]], p[f[2]]);
            let v = self.pos.len() as i32;

            for &i in f {
                self.push_vertex(p[i], n, Vec3::ZERO, Vec2::ZERO);
            }

            self.flags.push(tex | dir_flag(n));
            self.extra.push(0);
            self.quads.push([v, v + 1, v + 2, v + 3]);
        }
    }

    pub fn rotate_grid(&mut self, turns: i32, mut flags: u32) -> u32 {
        for _ in 0..turns.max(0) {
            for p in &mut self.ctrl {
                (p.x, p.z) = (1.0 - p.z, p.x);
            }

            flags = match flags {
                FACE_POS_Z => FACE_NEG_X,
                FACE_NEG_X => FACE_NEG_Z,
                FACE_NEG_Z => FACE_POS_X,
                FACE_POS_X => FACE_POS_Z,
                f => f,
            };
        }

        flags
    }

    pub fn face_by_direction(&mut self, dir: i32, off: Vec3) {
        let flags = match dir {
            0 => {
                self.set_corners([0.0, Q, 0.0], [1.0, Q, 0.0], [1.0, Q, 1.0], [0.0, Q, 1.0]);
                FACE_UP
            }
            5 => {
                self.set_corners([0.0, 0.875, 1.0], [1.0, 0.875, 1.0], [1.0, 0.875, 0.0], [0.0, 0.875, 0.0]);
                FACE_DOWN
            }
            _ => {
                self.set_corners([0.0, 1.0, Q], [1.0, 1.0, Q], [1.0, 0.0, Q], [0.0, 0.0, Q]);
                self.rotate_grid(dir - 1, FACE_POS_Z)
            }
        };

        for i in [0, 3, 15, 12] {
            self.ctrl[i] += off;
        }

        self.append_bezier(1, Some(Degree::Linear), 1, Some(Degree::Linear), flags | dir as u32);
        *self.extra.last_mut().unwrap() = 0x80;
    }

    pub fn inset_face(&mut self, dir: i32, mask: u32) {
        const E: f32 = 0.9375;

        let flags = match dir {
            0 => {
                let (mut p0, mut p3, mut p15, mut p12) = ([0.0, Q, 0.0], [1.0, Q, 0.0], [1.0, Q, 1.0], [0.0, Q, 1.0]);

                if mask & 2 != 0 {
                    (p0[2], p3[2]) = (Q, Q);
                }

                if mask & 4 != 0 {
                    (p3[0], p15[0]) = (E, E);
                }

                if mask & 8 != 0 {
                    (p15[2], p12[2]) = (E, E);
                }

                if mask & 16 != 0 {
                    (p0[0], p12[0]) = (Q, Q);
                }

                self.set_corners(p0, p3, p15, p12);
                FACE_UP
            }
            5 => {
                let (mut p0, mut p3, mut p15, mut p12) = ([0.0, 0.875, 1.0], [1.0, 0.875, 1.0], [1.0, 0.875, 0.0], [0.0, 0.875, 0.0]);

                if mask & 2 != 0 {
                    (p15[2], p12[2]) = (Q, Q);
                }

                if mask & 4 != 0 {
                    (p3[0], p15[0]) = (E, E);
                }

                if mask & 8 != 0 {
                    (p0[2], p3[2]) = (E, E);
                }

                if mask & 16 != 0 {
                    (p0[0], p12[0]) = (Q, Q);
                }

                self.set_corners(p0, p3, p15, p12);
                FACE_DOWN
            }
            _ => {
                self.set_corners([0.0, 1.0, Q], [1.0, 1.0, Q], [1.0, 0.0, Q], [0.0, 0.0, Q]);
                self.rotate_grid(dir - 1, FACE_POS_Z)
            }
        };

        self.append_bezier(1, Some(Degree::Linear), 1, Some(Degree::Linear), flags | dir as u32);
        *self.extra.last_mut().unwrap() = 0x80;
    }

    pub fn box_geometry(&mut self, turns: i32, mask: u32, codes: &[i32; 6], o: Vec3, s: Vec3) {
        for (r, f) in CUBE_FACES.iter().enumerate() {
            if mask & (1 << r) == 0 {
                continue;
            }

            let c = |i: usize| {
                let u = CUBE_CORNERS[f[i]];
                u * s + o
            };

            self.ctrl[0] = c(0);
            self.ctrl[3] = c(1);
            self.ctrl[15] = c(2);
            self.ctrl[12] = c(3);

            let rot = self.rotate_grid(turns, [FACE_DOWN, FACE_POS_Z, FACE_NEG_X, FACE_NEG_Z, FACE_POS_X, FACE_UP][r]);
            let mut code = codes[r];

            if (code - 1) as u32 <= 3 {
                code = ((code - 1 + turns) & 3) + 1;
            }

            if (code - 9) as u32 <= 3 {
                code = ((code - 9 + turns) & 3) + 9;
            }

            self.append_bezier(1, Some(Degree::Linear), 1, Some(Degree::Linear), code as u32 | rot);
        }
    }

    pub fn turn(&mut self, dims: IVec3) -> IVec3 {
        let d = dims.z as f32;

        for p in &mut self.pos {
            (p.x, p.z) = (d - p.z, p.x);
        }

        for n in self.normals.iter_mut().chain(self.tangents.iter_mut()) {
            (n.x, n.z) = (-n.z, n.x);
        }

        for f in &mut self.flags {
            let v = *f;
            let dir = v & (15 << 16);

            if dir == FACE_DOWN || dir == FACE_UP {
                *f = (v & !(15 << 12)) | (((v & (15 << 12)) + (1 << 12)) & (3 << 12));
                continue;
            }

            let code = v & 65535;
            let next = if code.wrapping_sub(1) <= 2 {
                code + 1
            } else if code == 4 {
                1
            } else {
                code
            };

            *f = match dir {
                0 => next,
                FACE_POS_Z => next | FACE_NEG_X,
                FACE_NEG_X => next | FACE_NEG_Z,
                FACE_NEG_Z => next | FACE_POS_X,
                FACE_POS_X => next | FACE_POS_Z,
                _ => v,
            };
        }

        IVec3::new(dims.z, dims.y, dims.x)
    }

    /// finalize_block_mesh: quads lying flat on a side of the block first (with a mask per side of the quads that
    /// don't), texture coordinates projected from the position for quads facing a direction, and the result.
    pub fn finalize(&mut self) -> BlockMesh {
        let mut k = 0usize;
        let mut side_masks = [0u32; 6];
        let mut face_side = vec![-1; self.quads.len()];

        for i in 0..self.quads.len() {
            let side = (0..6).rev().find(|&r| {
                let axis = [0, 1, 2, 2, 1, 0][r];
                let val = if r <= 2 { 0.0 } else { 1.0 };

                self.quads[i].iter().all(|&vi| self.pos[vi as usize][axis] == val)
            });

            if let Some(side) = side {
                self.quads.swap(k, i);
                self.flags.swap(k, i);
                self.extra.swap(k, i);

                side_masks[side] |= 1u32.wrapping_shl(k as u32);
                face_side[k] = side as i32;

                k += 1;
            }
        }

        let side_masks = side_masks.map(|m| !m);

        for (q, (&flags, extra)) in self.quads.iter().zip(self.flags.iter().zip(self.extra.iter_mut())) {
            let project: Option<fn(Vec3) -> Vec2> = match flags & (15 << 16) {
                FACE_NEG_X => Some(|p| Vec2::new(p.z, p.y)),
                FACE_POS_X => Some(|p| Vec2::new(1.0 - p.z, p.y)),
                FACE_DOWN => Some(|p| Vec2::new(p.x, p.z)),
                FACE_UP => Some(|p| Vec2::new(p.x, 1.0 - p.z)),
                FACE_NEG_Z => Some(|p| Vec2::new(1.0 - p.x, p.y)),
                FACE_POS_Z => Some(|p| Vec2::new(p.x, p.y)),
                _ => None,
            };

            if let Some(project) = project {
                for &vi in q {
                    self.uvs[vi as usize] = project(self.pos[vi as usize]);
                }
            }

            if flags & (15 << 16) == FACE_UP {
                *extra |= 1;
            }
        }

        BlockMesh {
            verts: self.pos.clone(),
            quads: self.quads.clone(),
            walls: self.walls.clone(),
            normals: self.normals.clone(),
            tangents: self.tangents.clone(),
            uvs: self.uvs.clone(),
            face_codes: self.flags.iter().map(|f| f & 0xfff).collect(),
            face_extra: self.extra.clone(),
            face_side,
            side_faces: k,
            side_masks,
        }
    }
}

pub fn dir_flag(n: Vec3) -> u32 {
    if n.y > 0.707 {
        FACE_UP
    } else if n.y < -0.707 {
        FACE_DOWN
    } else if n.x > 0.707 {
        FACE_POS_X
    } else if n.x < -0.707 {
        FACE_NEG_X
    } else if n.z > 0.707 {
        FACE_POS_Z
    } else {
        FACE_NEG_Z
    }
}

pub const Q: f32 = 0.0625;

pub const CUBE_CORNERS: [Vec3; 8] = [
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(1.0, 0.0, 0.0),
    Vec3::new(1.0, 0.0, 1.0),
    Vec3::new(0.0, 0.0, 1.0),
    Vec3::new(0.0, 1.0, 0.0),
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(0.0, 1.0, 1.0),
];
pub const CUBE_FACES: [[usize; 4]; 6] = [[3, 2, 1, 0], [7, 6, 2, 3], [4, 7, 3, 0], [5, 4, 0, 1], [6, 5, 1, 2], [4, 5, 6, 7]];
pub const CUBE_NORMALS: [Vec3; 6] = [Vec3::NEG_Y, Vec3::Z, Vec3::NEG_X, Vec3::NEG_Z, Vec3::X, Vec3::Y];
