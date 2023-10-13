//! Represents a block that is a part of a structure

use bevy::{
    prelude::{App, Deref, DerefMut},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockFace},
    registry::Registry,
};

use super::{
    coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType},
    Structure,
};

#[derive(Clone, Deref, DerefMut, Debug, Reflect, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// A block that is a part of a structure
///
/// This may not be valid when it is used.
pub struct StructureBlock(pub BlockCoordinate);

impl From<StructureBlock> for (CoordinateType, CoordinateType, CoordinateType) {
    fn from(val: StructureBlock) -> Self {
        (val.x, val.y, val.z)
    }
}

impl From<&StructureBlock> for (CoordinateType, CoordinateType, CoordinateType) {
    fn from(val: &StructureBlock) -> Self {
        (val.x, val.y, val.z)
    }
}

impl StructureBlock {
    #[inline]
    /// Gets the x position
    pub fn x(&self) -> CoordinateType {
        self.x
    }
    #[inline]
    /// Gets the y position
    pub fn y(&self) -> CoordinateType {
        self.y
    }
    #[inline]
    /// Gets the z position
    pub fn z(&self) -> CoordinateType {
        self.z
    }

    /// Creates a structure block
    pub fn new(coords: BlockCoordinate) -> Self {
        Self(coords)
    }

    #[inline]
    /// Returns this block's top facing face
    pub fn block_up(&self, structure: &Structure) -> BlockFace {
        structure.block_rotation(self.0)
    }

    #[inline]
    /// Returns the numeric block id - this returns air if the block is not loaded
    pub fn block_id(&self, structure: &Structure) -> u16 {
        structure.block_id_at(self.0)
    }

    #[inline]
    /// Returns the block that is at this location - this returns air if the block is not loaded
    pub fn block<'a>(&self, structure: &Structure, blocks: &'a Registry<Block>) -> &'a Block {
        blocks.from_numeric_id(self.block_id(structure))
    }

    #[inline]
    /// The chunk that contains this block's coordinates as (x, y, z).
    pub fn coords(&self) -> BlockCoordinate {
        self.0
    }

    #[inline]
    /// The chunk that contains this block's coordinates as (x, y, z).
    pub fn chunk_coords(&self) -> ChunkCoordinate {
        ChunkCoordinate::for_block_coordinate(self.0)
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<StructureBlock>();
}
