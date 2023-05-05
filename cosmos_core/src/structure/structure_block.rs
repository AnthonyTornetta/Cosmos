//! Represents a block that is a part of a structure

use bevy::{
    prelude::App,
    reflect::{FromReflect, Reflect},
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockFace},
    registry::Registry,
};

use super::{chunk::CHUNK_DIMENSIONS, Structure};

#[derive(
    Clone, Debug, FromReflect, Reflect, Copy, PartialEq, Eq, Default, Serialize, Deserialize,
)]
/// A block that is a part of a structure
///
/// This may not be valid when it is used.
pub struct StructureBlock {
    /// Block x position
    pub x: usize,
    /// Block y position
    pub y: usize,
    /// Block z position
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
    /// Gets the x position
    pub fn x(&self) -> usize {
        self.x
    }
    #[inline]
    /// Gets the y position
    pub fn y(&self) -> usize {
        self.y
    }
    #[inline]
    /// Gets the z position
    pub fn z(&self) -> usize {
        self.z
    }

    /// Creates a structure block
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }

    #[inline]
    /// Returns this block's top facing face
    pub fn block_up(&self, structure: &Structure) -> BlockFace {
        structure.block_rotation(self.x, self.y, self.z)
    }

    #[inline]
    /// Returns the numeric block id - this returns air if the block is not loaded
    pub fn block_id(&self, structure: &Structure) -> u16 {
        structure.block_id_at(self.x, self.y, self.z)
    }

    #[inline]
    /// Returns the block that is at this location - this returns air if the block is not loaded
    pub fn block<'a>(&self, structure: &Structure, blocks: &'a Registry<Block>) -> &'a Block {
        blocks.from_numeric_id(self.block_id(structure))
    }

    #[inline]
    /// The chunk that contains this block's x coordinate
    pub fn chunk_coord_x(&self) -> usize {
        self.x / CHUNK_DIMENSIONS
    }

    #[inline]
    /// The chunk that contains this block's y coordinate
    pub fn chunk_coord_y(&self) -> usize {
        self.y / CHUNK_DIMENSIONS
    }

    #[inline]
    /// The chunk that contains this block's z coordinate
    pub fn chunk_coord_z(&self) -> usize {
        self.z / CHUNK_DIMENSIONS
    }

    #[inline]
    /// The chunk that contains this block's coordinates as (x, y, z).
    pub fn chunk_coords(&self) -> (usize, usize, usize) {
        (
            self.chunk_coord_x(),
            self.chunk_coord_y(),
            self.chunk_coord_z(),
        )
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<StructureBlock>();
}
