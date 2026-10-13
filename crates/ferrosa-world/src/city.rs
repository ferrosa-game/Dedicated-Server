use ferrosa_map::file_types::sbc::CityFileSBC;
use glam::IVec3;

use crate::{
    area::{AreaGrid, BlockDims, CUBE, MESH, TYPE_MASK},
    ground::Ground,
};

pub struct BlockType {
    pub value: u32,
    /// Each texture collection's 8 materials (city_resolve_block_and_texture_indices).
    pub textures: Vec<[u16; 8]>,
}

/// The material named `name`, the last that matches, or 0.
fn material(materials: &[String], name: &str) -> u16 {
    materials.iter().enumerate().rfind(|(_, m)| m.as_str() == name).map_or(0, |(k, _)| k as u16)
}

pub fn resolve_block_types(city: &CityFileSBC, block_names: &[String], materials: &[String]) -> Vec<BlockType> {
    city.blocktypes
        .iter()
        .map(|bt| {
            let names = if bt.texture_names_12.is_empty() { &bt.texture_names_11 } else { &bt.texture_names_12 };
            let textures = names.chunks(8).map(|c| std::array::from_fn(|k| c.get(k).map_or(0, |n| material(materials, n.as_str())))).collect();
            let name = bt.name.as_str();
            let mut value = CUBE;

            if !name.is_empty() {
                for (id, n) in block_names.iter().enumerate() {
                    if n == name {
                        value = id as u32 | MESH;
                    }
                }
            }

            BlockType { value, textures }
        })
        .collect()
}

pub fn place_sector_block(grid: &mut AreaGrid, types: &[BlockType], dims: &dyn BlockDims, x: i32, y: i32, z: i32, packed: u32) {
    let r = (packed >> 24) & 3;
    let Some(bt) = types.get((packed & 4095) as usize) else { return };
    let set = bt.textures.get((((packed >> 16) & 31) as usize).wrapping_sub(1)).copied().unwrap_or([0; 8]);

    if bt.value & TYPE_MASK == CUBE {
        let turn = (r.wrapping_neg() & 3) as usize;
        let sides: [u16; 4] = std::array::from_fn(|k| set[1 + ((turn + k) & 3)]);
        grid.create_block(x, y, z, CUBE, [set[0], sides[0], sides[1], sides[2], sides[3], set[5], set[6], set[7]], dims);
    } else {
        grid.create_block(x, y, z, bt.value + r, set, dims);
    }
}

pub fn instantiate_sectors(city: &CityFileSBC, types: &[BlockType], grid: &mut AreaGrid, ground: &mut Ground, dims: &dyn BlockDims) {
    for sector in &city.sectors {
        let base = sector.pos.as_ivec3() * 8;

        for y in 0..8 {
            for z in 0..8 {
                for x in 0..8 {
                    let packed = sector.block_type_indices.get((y * 64 + z * 8 + x) as usize).copied().unwrap_or(0);

                    if packed == 0 {
                        continue;
                    }

                    let p = base + IVec3::new(x, y, z);

                    place_sector_block(grid, types, dims, p.x, p.y, p.z, packed);
                    ground.stamp_roadmap(p.x, p.z, p.y as f32 * 4.0);
                }
            }
        }
    }
}
