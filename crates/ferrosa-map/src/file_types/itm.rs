use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::Path};

use super::LoaderError;
use crate::br_vec3;
use binrw::BinRead;
use glam::Vec3;

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct ItmPoint {
    pub kind: i32,
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct ItmVertex {
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    pub uv: [f32; 2],
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct ItmFace {
    pub count: u32,
    #[br(count = count)]
    pub indices: Vec<i32>,
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
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
