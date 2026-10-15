use binrw::BinRead;
use glam::Vec3;

use crate::br_vec3;

#[derive(BinRead)]
pub struct SkinnedModelWeight {
    #[br(parse_with = br_vec3)]
    pub offset: Vec3,
    pub weight: f32,
}

#[derive(BinRead)]
#[br(import(bone_count: u32))]
pub struct SkinnedModelVertex {
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    #[br(count = bone_count)]
    pub weights: Vec<SkinnedModelWeight>,
    pub u: f32,
    pub v: f32,
}

#[derive(BinRead)]
#[br(little, magic = b"CMod")]
pub struct SkinnedModel {
    #[br(assert(version == 2))]
    pub version: u32,

    pub bone_count: u32,
    #[br(parse_with = binrw::helpers::count_with(bone_count as usize, br_vec3))]
    pub bones: Vec<Vec3>,

    pub vertex_count: u32,
    #[br(args { count: vertex_count as usize, inner: (bone_count,) })]
    pub vertices: Vec<SkinnedModelVertex>,

    pub face_count: u32,
    #[br(count = face_count)]
    pub faces: Vec<[u32; 3]>,
}