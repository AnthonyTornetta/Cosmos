use std::slice::Iter;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    block::{blocks::AIR_BLOCK_ID, Block, BlockFace},
    registry::{identifiable::Identifiable, Registry},
    structure::chunk::CHUNK_DIMENSIONS,
};

use super::{
    chunk::BlockInfo,
    coordinates::{ChunkBlockCoordinate, Coordinate, CoordinateType},
};

#[derive(Debug, Reflect, Serialize, Deserialize)]
/// A generic way of storing blocks and their information
pub struct BlockStorage {
    blocks: Vec<u16>,
    block_info: Vec<BlockInfo>,
    non_air_blocks: u32,
    width: CoordinateType,
    height: CoordinateType,
    length: CoordinateType,
}

impl BlockStorage {
    /// A generic way of storing blocks and their information
    ///
    /// - `width` the number of blocks in the x direction [0, width)
    /// - `height` the number of blocks in the y direction [0, height)
    /// - `length` the number of blocks in the z direction [0, length)
    pub fn new(width: CoordinateType, height: CoordinateType, length: CoordinateType) -> Self {
        let n_blocks = width * height * length;

        Self {
            blocks: vec![0; n_blocks as usize],
            block_info: vec![BlockInfo::default(); n_blocks as usize],
            non_air_blocks: 0,
            width,
            height,
            length,
        }
    }

    #[inline(always)]
    fn flatten(coords: ChunkBlockCoordinate) -> usize {
        coords.flatten(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS)
    }

    #[inline(always)]
    /// Debug asserts that coordinates are within a chunk
    ///
    /// Will panic in debug mode if they are not
    pub fn debug_assert_is_within_blocks(&self, coords: ChunkBlockCoordinate) {
        debug_assert!(
            coords.x < self.width && coords.y < self.height && coords.z < self.length,
            "{} < {} && {} < {} && {} < {} failed",
            coords.x,
            self.width,
            coords.y,
            self.height,
            coords.z,
            self.length
        );
    }

    #[inline]
    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    pub fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_up: BlockFace) {
        self.set_block_at_from_id(coords, b.id(), block_up);
    }

    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    pub fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_up: BlockFace) {
        self.debug_assert_is_within_blocks(coords);

        let index = Self::flatten(coords);

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
    /// Gets the block at this location. Air is returned for empty blocks.
    pub fn block_at(&self, coords: ChunkBlockCoordinate) -> u16 {
        self.blocks[Self::flatten(coords)]
    }

    #[inline]
    /// Gets the block's rotation at this location
    pub fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockFace {
        self.block_info[Self::flatten(coords)].get_rotation()
    }

    #[inline]
    /// Returns true if this chunk only contains air.
    pub fn is_empty(&self) -> bool {
        self.non_air_blocks == 0
    }

    /// Returns the iterator for every block in the chunk
    pub fn blocks(&self) -> Iter<u16> {
        self.blocks.iter()
    }

    /// Returns the iterator for all the block info of the chunk
    pub fn block_info_iterator(&self) -> Iter<BlockInfo> {
        self.block_info.iter()
    }

    #[inline]
    /// Returns true if the block at these coordinates is a full block (1x1x1 cube). This is not determined
    /// by the model, but rather the flags the block is constructed with.
    pub fn has_full_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        blocks.from_numeric_id(self.block_at(coords)).is_full()
    }

    #[inline]
    /// Returns true if the block at this location is see-through. This is not determined from the block's texture, but
    /// rather the flags the block was constructed with.
    pub fn has_see_through_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        blocks.from_numeric_id(self.block_at(coords)).is_see_through()
    }

    #[inline]
    /// Returns true if the block at this location is not air.
    pub fn has_block_at(&self, coords: ChunkBlockCoordinate) -> bool {
        self.block_at(coords) != AIR_BLOCK_ID
    }

    /// Sets every block within this to be this block + rotation
    pub fn fill(&mut self, block: &Block, block_up: BlockFace) {
        for z in 0..self.length {
            for y in 0..self.height {
                for x in 0..self.width {
                    self.set_block_at((x, y, z).into(), block, block_up);
                }
            }
        }
    }
}
