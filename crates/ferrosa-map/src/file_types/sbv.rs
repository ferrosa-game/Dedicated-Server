use std::{io::Cursor, path::Path};

use binrw::BinRead;
use glam::Vec3;
use crate::br_vec3;

use super::LoaderError;

#[derive(BinRead, Debug, Clone, Copy)]
#[brw(little)]
pub struct SbvNode {
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    pub weight: f32,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[brw(little)]
pub struct SbvEdge {
    pub kind: i32,
    pub a: i32,
    pub b: i32,
}

#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct SbvFace {
    pub count: u32,
    #[br(count = count)]
    pub indices: Vec<i32>,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[brw(little)]
#[br(import(version: u32))]
pub struct SbvSubshapeNode {
    pub id: i32,
    #[br(if(version > 1, [0.0; 4]))]
    pub weights: [f32; 4],
}

#[derive(BinRead, Debug, Clone)]
#[brw(little)]
#[br(import(version: u32))]
pub struct SbvSubshape {
    #[br(if(version > 2, 0))]
    pub flag: i32,
    pub count: u32,
    #[br(args { count: count as usize, inner: (version,) })]
    pub nodes: Vec<SbvSubshapeNode>,
    #[br(if(version > 1, [0; 4]))]
    pub quad: [i32; 4],
}

#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct SbvPart {
    pub count: u32,
    #[br(parse_with = binrw::helpers::count_with(count as usize, br_vec3))]
    pub points: Vec<Vec3>,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[brw(little)]
pub struct SbvWheel {
    pub kind: i32,
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    pub aux: i32,
}

#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct VehicleBodyFile {
    pub version: u32,
    #[br(if(version > 4), parse_with = br_vec3, map = Some)]
    pub offset: Option<Vec3>,

    pub node_count: u32,
    #[br(count = node_count)]
    pub nodes: Vec<SbvNode>,

    pub edge_count: u32,
    #[br(count = edge_count)]
    pub edges: Vec<SbvEdge>,

    pub face_count: u32,
    #[br(count = face_count)]
    pub faces: Vec<SbvFace>,

    pub vertex_count: u32,
    #[br(parse_with = binrw::helpers::count_with(vertex_count as usize, br_vec3))]
    pub vertices: Vec<Vec3>,

    pub subshape_count: u32,
    #[br(args { count: subshape_count as usize, inner: (version,) })]
    pub subshapes: Vec<SbvSubshape>,

    pub part_count: u32,
    #[br(count = part_count)]
    pub parts: Vec<SbvPart>,

    pub wheel_count: u32,
    #[br(count = wheel_count)]
    pub wheels: Vec<SbvWheel>,
}

impl VehicleBodyFile {
    pub fn load(path: &Path) -> Result<Self, LoaderError> {
        let bytes = std::fs::read(path)?;
        Ok(VehicleBodyFile::read(&mut Cursor::new(bytes))?)
    }
}
