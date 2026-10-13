use std::{io::Cursor, path::Path};
use crate::br_vec3;

use binrw::BinRead;
use glam::Vec3;

use super::LoaderError;
use crate::Char64;

/// One item of an item set: its type by name, where it sits in the cell and how it is turned.
#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct SisEntry {
    pub name: Char64,
    #[br(parse_with = br_vec3)]
    pub pos: Vec3,
    #[br(map = |a: [[f32; 3]; 3]| a.map(Vec3::from_array))]
    pub rot: [Vec3; 3],
}

/// An item set file (data/itemset/*.sis, load_item_set): items placed together in a level cell.
#[derive(BinRead, Debug, Clone)]
#[brw(little)]
pub struct ItemSetFile {
    pub version: i32,
    pub count: u32,
    #[br(count = count)]
    pub entries: Vec<SisEntry>,
}

impl ItemSetFile {
    pub fn load(path: &Path) -> Result<Self, LoaderError> {
        let bytes = std::fs::read(path)?;
        Ok(ItemSetFile::read(&mut Cursor::new(bytes))?)
    }
}
