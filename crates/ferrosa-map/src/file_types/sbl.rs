use crate::{br_uvec3, br_vec3};
use binrw::BinRead;
use glam::{UVec3, Vec3};

#[derive(BinRead, Debug, Clone)]
pub struct BlockSurfaceVertex {
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    #[br(count = 2)]
    pub tex_coord: Vec<f32>,
}

#[derive(BinRead, Debug, Clone)]
pub struct BlockSurface {
    pub order_x: u32,
    pub order_y: u32,
    pub tess_x: u32,
    pub tess_y: u32,
    pub texture: u32,
    pub project_texture: u32,

    pub vertex_data: [[BlockSurfaceVertex; 4]; 4],
}

#[derive(BinRead, Debug, Clone)]
pub struct BlockBox {
    #[br(parse_with = binrw::helpers::count_with(8, br_vec3))]
    pub pos: Vec<Vec3>,
    #[br(count = 6)]
    pub texture: Vec<u32>,
    pub side_flag: u32,
}

#[derive(BinRead, Debug, Clone)]
#[br(little)]
pub struct BlockFile {
    pub version: u32,
    #[br(parse_with = br_uvec3)]
    pub size: UVec3,
    pub floor: u32,
    pub ceiling: u32,
    pub wall_pz: u32,
    pub wall_nx: u32,
    pub wall_px: u32,
    pub wall_nz: u32,

    pub surface_count: u32,
    #[br(count = surface_count)]
    pub block_surface: Vec<BlockSurface>,

    pub box_count: u32,
    #[br(count = box_count)]
    pub boxes: Vec<BlockBox>,
}
