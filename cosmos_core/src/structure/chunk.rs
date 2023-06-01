//! Represents a fixed region of blocks.
//!
//! These blocks can be updated.

use std::slice::Iter;

use crate::block::blocks::AIR_BLOCK_ID;
use crate::block::hardness::BlockHardness;
use crate::block::{Block, BlockFace};
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use crate::utils::array_utils::flatten;
use bevy::prelude::{Component, Entity, Vec3};
use bevy::reflect::{FromReflect, Reflect};
use serde::{Deserialize, Serialize};

use super::block_health::BlockHealth;

/// The number of blocks a chunk can have in the x/y/z directions.
///
/// A chunk contains `CHUNK_DIMENSIONS`^3 blocks total.
pub const CHUNK_DIMENSIONS: usize = 32;

/// Short for `CHUNK_DIMENSIONS as f32`
pub const CHUNK_DIMENSIONSF: f32 = CHUNK_DIMENSIONS as f32;

/// The number of blocks a chunk contains (`CHUNK_DIMENSIONS^3`)
const N_BLOCKS: usize = CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS;

#[derive(Debug, Reflect, FromReflect, Serialize, Deserialize)]
/// Stores a bunch of blocks, information about those blocks, and where they are in the structure.
pub struct Chunk {
    x: usize,
    y: usize,
    z: usize,
    blocks: Vec<u16>,
    block_info: Vec<BlockInfo>,

    block_health: BlockHealth,

    non_air_blocks: usize,
}

impl Chunk {
    /// Creates a chunk containing all air blocks.
    ///
    /// * `x` The x chunk location in the structure
    /// * `y` The y chunk location in the structure
    /// * `z` The z chunk location in the structure
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self {
            x,
            y,
            z,
            blocks: vec![0; N_BLOCKS],
            block_info: vec![BlockInfo::default(); N_BLOCKS],
            block_health: BlockHealth::default(),
            non_air_blocks: 0,
        }
    }

    #[inline]
    /// The position in the structure x
    pub fn structure_x(&self) -> usize {
        self.x
    }

    #[inline]
    /// The position in the structure y
    pub fn structure_y(&self) -> usize {
        self.y
    }

    #[inline]
    /// The position in the structure z
    pub fn structure_z(&self) -> usize {
        self.z
    }

    #[inline]
    /// Returns true if this chunk only contains air
    pub fn is_empty(&self) -> bool {
        self.non_air_blocks == 0
    }

    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    pub fn set_block_at(&mut self, x: usize, y: usize, z: usize, b: &Block, block_up: BlockFace) {
        let index = flatten(x, y, z, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS);
        let id = b.id();

        self.block_health.reset_health(x, y, z);

        self.block_info[index].set_rotation(block_up);

        if self.blocks[index] != id {
            if self.blocks[index] == AIR_BLOCK_ID {
                self.non_air_blocks += 1;
            } else if id == AIR_BLOCK_ID {
                self.non_air_blocks -= 1;
            }

            self.blocks[index] = id;
        }
    }

    #[inline]
    /// Returns true if the block at this location is see-through. This is not determined from the block's texture, but
    /// rather the flags the block was constructed with.
    pub fn has_see_through_block_at(
        &self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Registry<Block>,
    ) -> bool {
        blocks
            .from_numeric_id(self.block_at(x, y, z))
            .is_see_through()
    }

    #[inline]
    /// Returns true if the block at this location is not air.
    pub fn has_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_at(x, y, z) != AIR_BLOCK_ID
    }

    #[inline]
    /// Gets the block at this location. Air is returned for empty blocks.
    pub fn block_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.blocks[flatten(x, y, z, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS)]
    }

    #[inline]
    /// Gets the block's rotation at this location
    pub fn block_rotation(&self, x: usize, y: usize, z: usize) -> BlockFace {
        self.block_info[flatten(x, y, z, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS)].get_rotation()
    }

    #[inline]
    /// Returns true if the block at these coordinates is a full block (1x1x1 cube). This is not determined
    /// by the model, but rather the flags the block is constructed with.
    pub fn has_full_block_at(
        &self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Registry<Block>,
    ) -> bool {
        blocks.from_numeric_id(self.block_at(x, y, z)).is_full()
    }

    /// Calculates the block coordinates used in something like `Chunk::block_at` from their f32 coordinates relative to the chunk's center.
    pub fn relative_coords_to_block_coords(&self, relative: &Vec3) -> (usize, usize, usize) {
        (
            (relative.x + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
            (relative.y + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
            (relative.z + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
        )
    }

    /// Gets the block's health at that given coordinate
    /// * `x/y/z`: block coordinate
    /// * `block_hardness`: The hardness for the block at those coordinates
    pub fn get_block_health(
        &self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
    ) -> f32 {
        self.block_health.get_health(x, y, z, block_hardness)
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// * `x/y/z` Block coordinates
    /// * `block_hardness` The hardness for that block
    /// * `amount` The amount of damage to take - cannot be negative
    ///
    /// **Returns:** true if that block was destroyed, false if not
    pub fn block_take_damage(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block_hardness: &BlockHardness,
        amount: f32,
    ) -> bool {
        self.block_health
            .take_damage(x, y, z, block_hardness, amount)
    }

    /// Returns the iterator for every block in the chunk
    pub fn blocks(&self) -> Iter<u16> {
        self.blocks.iter()
    }

    /// Returns the iterator for all the block info of the chunk
    pub fn block_info_iterator(&self) -> Iter<BlockInfo> {
        self.block_info.iter()
    }
}

#[derive(
    Debug, Default, Reflect, FromReflect, Serialize, Deserialize, Clone, Copy, PartialEq, Eq,
)]
/// This represents the information for a block. The first 3 bits are reserved for rotation data.
///
/// All other bits can be used for anything else
pub struct BlockInfo(u8);

impl BlockInfo {
    #[inline]
    /// Gets the rotation data
    ///
    /// This will return which BlockFace represents the UP direction (no rotation is BlockFace::Top)
    pub fn get_rotation(&self) -> BlockFace {
        BlockFace::from_index((self.0 & 0b111) as usize)
    }

    /// Sets the rotation data
    ///
    /// This should be the BlockFace that represents the UP direction (no rotation is BlockFace::Top)
    pub fn set_rotation(&mut self, rotation: BlockFace) {
        self.0 = self.0 & !0b111 | rotation.index() as u8;
    }
}

/// Represents a child of a structure that represents a chunk
#[derive(Debug, Reflect, FromReflect, Component)]
pub struct ChunkEntity {
    /// The entity of the structure this is a part of
    pub structure_entity: Entity,
    /// The chunk's position in the structure (x, y, z)
    pub chunk_location: (usize, usize, usize),
}
