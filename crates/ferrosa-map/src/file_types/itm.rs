use std::{io::Cursor, path::Path};

use super::LoaderError;
use crate::br_vec3;
use binrw::BinRead;
use glam::Vec3;

/// A marked point of the item (0 the muzzle, 1 the right hand, 2 the left hand, 4 the second hand's grip).
#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct ItmPoint {
    pub kind: i32,
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
}

/// A render vertex and its texture coordinate.
#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct ItmVertex {
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    pub uv: [f32; 2],
}

/// A render face: three or four vertex indices.
#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct ItmFace {
    pub count: u32,
    #[br(count = count)]
    pub indices: Vec<i32>,
}

/// An item model file (data/*.itm, load_item_file): its centre, inertia, marked points and render mesh.
#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct ItmFile {
    pub version: i32,
    #[br(parse_with = br_vec3)]
    pub center: Vec3,
    #[br(parse_with = br_vec3)]
    pub inertia: Vec3,
    pub point_count: u32,
    #[br(count = point_count)]
    pub points: Vec<ItmPoint>,
    pub vert_count: u32,
    #[br(count = vert_count)]
    pub verts: Vec<ItmVertex>,
    pub face_count: u32,
    #[br(count = face_count)]
    pub faces: Vec<ItmFace>,
}

impl ItmFile {
    pub fn load(path: &Path) -> Result<Self, LoaderError> {
        let bytes = std::fs::read(path)?;
        Ok(ItmFile::read(&mut Cursor::new(bytes))?)
    }
}
