use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::Path};

use binrw::BinRead;
use glam::Vec3;

use crate::br_vec3;

use super::LoaderError;

const VERSION: i32 = 3;
const MESH_CHUNK: i32 = 0;
const COLLISION_CHUNK: i32 = 1;

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct It3Corner {
    pub index: i32,
    pub uv: [f32; 2],
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct It3Face {
    pub count: u32,
    #[br(count = count)]
    pub corners: Vec<It3Corner>,
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct It3Mesh {
    pub sub_version: i32,
    pub vert_count: u32,
    #[br(parse_with = binrw::helpers::count_with(vert_count as usize, br_vec3))]
    pub verts: Vec<Vec3>,
    pub face_count: u32,
    #[br(count = face_count)]
    pub faces: Vec<It3Face>,
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct It3Indices {
    pub count: u32,
    #[br(count = count)]
    pub items: Vec<i32>,
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct It3Group {
    #[br(parse_with = br_vec3)]
    #[serde(with = "super::glb::any_vec3")]
    pub center: Vec3,
    pub verts: It3Indices,
    pub face_count: u32,
    #[br(count = face_count)]
    pub faces: Vec<It3Indices>,
}

#[derive(BinRead, Serialize, Deserialize, Debug, Clone)]
#[brw(little)]
pub struct It3Collision {
    pub sub_version: i32,
    pub vert_count: u32,
    #[br(parse_with = binrw::helpers::count_with(vert_count as usize, br_vec3))]
    pub verts: Vec<Vec3>,
    pub group_count: u32,
    #[br(count = group_count)]
    pub groups: Vec<It3Group>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct It3File {
    pub mesh: Option<It3Mesh>,
    pub collision: Option<It3Collision>,
}

impl It3File {
    pub fn load(path: &Path) -> Result<Self, LoaderError> {
        let bytes = std::fs::read(path)?;
        let mut r = Cursor::new(bytes);
        let version = i32::read_le(&mut r)?;

        if version != VERSION {
            return Ok(It3File { mesh: None, collision: None });
        }

        let mut chunk = i32::read_le(&mut r)?;

        let mesh = if chunk == MESH_CHUNK {
            let m = It3Mesh::read(&mut r)?;
            chunk = i32::read_le(&mut r)?;
            Some(m)
        } else {
            None
        };

        let collision = if chunk == COLLISION_CHUNK { Some(It3Collision::read(&mut r)?) } else { None };

        Ok(It3File { mesh, collision })
    }
}
