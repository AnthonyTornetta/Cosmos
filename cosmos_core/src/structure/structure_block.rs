use bevy::{
    prelude::App,
    reflect::{FromReflect, Reflect},
};
use serde::{Deserialize, Serialize};

use crate::{block::Block, registry::Registry};

use super::{chunk::CHUNK_DIMENSIONS, Structure};

#[derive(
    Clone, Debug, FromReflect, Reflect, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
pub struct StructureBlock {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl From<StructureBlock> for (usize, usize, usize) {
    fn from(val: StructureBlock) -> Self {
        (val.x, val.y, val.z)
    }
}

impl From<&StructureBlock> for (usize, usize, usize) {
    fn from(val: &StructureBlock) -> Self {
        (val.x, val.y, val.z)
    }
}

impl StructureBlock {
    #[inline]
    pub fn x(&self) -> usize {
        self.x
    }
    #[inline]
    pub fn y(&self) -> usize {
        self.y
    }
    #[inline]
    pub fn z(&self) -> usize {
        self.z
    }

    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn block_id(&self, structure: &Structure) -> u16 {
        structure.block_id_at(self.x, self.y, self.z)
    }

    #[inline]
    pub fn block<'a>(&self, structure: &Structure, blocks: &'a Registry<Block>) -> &'a Block {
        blocks.from_numeric_id(self.block_id(structure))
    }

    #[inline]
    pub fn chunk_coord_x(&self) -> usize {
        self.x / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coord_y(&self) -> usize {
        self.y / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coord_z(&self) -> usize {
        self.z / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coords(&self) -> (usize, usize, usize) {
        (
            self.chunk_coord_x(),
            self.chunk_coord_y(),
            self.chunk_coord_z(),
        )
    }
}

pub(crate) fn register(app: &mut App) {
    app.register_type::<StructureBlock>();
}
