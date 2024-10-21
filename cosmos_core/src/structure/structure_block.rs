//! Represents a block that is a part of a structure

use bevy::{
    prelude::{App, Entity},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{block_rotation::BlockRotation, Block},
    registry::Registry,
};

use super::{
    coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType},
    Structure,
};

#[derive(Clone, Debug, Reflect, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// A block that is a part of a structure.
///
/// This is really just a wrapper around a BlockCoordinate, but this implies there is (or at least was) a block here.
/// This may not be valid when it is used.
pub struct StructureBlock {
    coordinate: BlockCoordinate,
    structure: Entity,
}

impl StructureBlock {
    #[inline]
    /// Gets the x position
    pub fn x(&self) -> CoordinateType {
        self.coordinate.x
    }
    #[inline]
    /// Gets the y position
    pub fn y(&self) -> CoordinateType {
        self.coordinate.y
    }
    #[inline]
    /// Gets the z position
    pub fn z(&self) -> CoordinateType {
        self.coordinate.z
    }

    /// Creates a structure block
    pub fn new(coordinate: BlockCoordinate, structure: Entity) -> Self {
        Self { coordinate, structure }
    }

    #[inline]
    /// Returns this block's top facing face
    pub fn block_up(&self, structure: &Structure) -> BlockRotation {
        structure.block_rotation(self.coordinate)
    }

    #[inline]
    /// Returns the numeric block id - this returns air if the block is not loaded
    pub fn block_id(&self, structure: &Structure) -> u16 {
        structure.block_id_at(self.coordinate)
    }

    #[inline]
    /// Returns the block that is at this location - this returns air if the block is not loaded
    pub fn block<'a>(&self, structure: &Structure, blocks: &'a Registry<Block>) -> &'a Block {
        blocks.from_numeric_id(self.block_id(structure))
    }

    #[inline]
    /// The chunk that contains this block's coordinates as (x, y, z).
    pub fn coords(&self) -> BlockCoordinate {
        self.coordinate
    }

    #[inline]
    /// The chunk that contains this block's coordinates as (x, y, z).
    pub fn chunk_coords(&self) -> ChunkCoordinate {
        ChunkCoordinate::for_block_coordinate(self.coordinate)
    }

    #[inline]
    /// Returns the entity this block is for
    pub fn structure(&self) -> Entity {
        self.structure
    }

    /// Sets the structure this belongs to to this entity
    pub fn set_structure(&mut self, e: Entity) {
        self.structure = e;
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<StructureBlock>();
}
