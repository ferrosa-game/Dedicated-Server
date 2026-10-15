use binrw::BinRead;
use glam::Vec3;

use crate::br_vec3;

#[derive(BinRead)]
#[br(import(version: u32))]
pub struct ItemModelVertex {
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    #[br(if(version >= 3))]
    pub u: f32,
    #[br(if(version >= 3))]
    pub v: f32,
    pub padding: u32,
}

#[derive(BinRead)]
#[br(import(version: u32))]
pub struct ItemModelFace {
    pub num_vertices: u32,
    #[br(count = num_vertices)]
    pub vertex_ids: Vec<u32>,

    _unk: u32,
    #[br(if(version > 1))]
    pub skip: u32,
}

#[derive(BinRead)]
#[br(little, magic = b"CMod")]
pub struct ItemModel {
    #[br(assert((1..=3).contains(&version)))]
    pub version: u32,

    pub vertex_count: u32,
    #[br(args { count: vertex_count as usize, inner: (version,) })]
    pub vertices: Vec<ItemModelVertex>,

    pub face_count: u32,
    #[br(args { count: face_count as usize, inner: (version,) })]
    pub faces: Vec<ItemModelFace>,
}