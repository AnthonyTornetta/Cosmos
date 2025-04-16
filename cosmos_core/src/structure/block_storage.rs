//! Represents a generic way of storing blocks

use std::slice::Iter;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, block_rotation::BlockRotation, blocks::AIR_BLOCK_ID},
    registry::{Registry, identifiable::Identifiable},
    structure::chunk::CHUNK_DIMENSIONS,
};

use super::{
    chunk::BlockInfo,
    coordinates::{ChunkBlockCoordinate, Coordinate, CoordinateType},
};

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, PartialEq, Eq)]
/// A generic way of storing blocks and their information
pub struct BlockStorage {
    blocks: Vec<u16>,
    block_info: Vec<BlockInfo>,
    non_air_blocks: u32,
    width: CoordinateType,
    height: CoordinateType,
    length: CoordinateType,
}

/// Something that stores a bunch of blocks that are next to each other.
///
/// For example, a `Chunk`.
pub trait BlockStorer {
    /// Debug asserts that coordinates are within a chunk
    ///
    /// Will panic in debug mode if they are not
    fn debug_assert_is_within_blocks(&self, coords: ChunkBlockCoordinate);

    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_rotation: BlockRotation);

    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_rotation: BlockRotation);

    /// Gets the block at this location. Air is returned for empty blocks.
    fn block_at(&self, coords: ChunkBlockCoordinate) -> u16;

    /// Gets the block's rotation at this location
    fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockRotation;

    /// Returns true if this chunk only contains air.
    fn is_empty(&self) -> bool;

    /// Returns the iterator for every block in the chunk
    fn blocks(&self) -> Iter<u16>;

    /// Returns the iterator for all the block info of the chunk
    fn block_info_iterator(&self) -> Iter<BlockInfo>;

    /// Returns the small block information storage (for example, rotation) for this block within the chunk.
    fn block_info_at(&self, coords: ChunkBlockCoordinate) -> BlockInfo;

    /// Sets the small block information storage (for example, rotation) for this block within the chunk.
    fn set_block_info_at(&mut self, coords: ChunkBlockCoordinate, block_info: BlockInfo);

    /// Returns true if the block at these coordinates is a full block (1x1x1 cube). This is not determined
    /// by the model, but rather the flags the block is constructed with.
    fn has_full_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool;

    /// Returns true if the block at this location is see-through. This is not determined from the block's texture, but
    /// rather the flags the block was constructed with.
    fn has_see_through_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool;

    /// Returns true if the block at this location is not air.
    fn has_block_at(&self, coords: ChunkBlockCoordinate) -> bool;

    /// Returns true if the coordinates are within this block storage
    fn is_within_blocks(&self, coords: ChunkBlockCoordinate) -> bool;
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

    /// Sets every block within this to be this block + rotation
    pub fn fill(&mut self, block: &Block, block_rotation: BlockRotation) {
        for z in 0..self.length {
            for y in 0..self.height {
                for x in 0..self.width {
                    self.set_block_at((x, y, z).into(), block, block_rotation);
                }
            }
        }
    }
}

impl BlockStorer for BlockStorage {
    #[inline(always)]
    fn debug_assert_is_within_blocks(&self, coords: ChunkBlockCoordinate) {
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
    fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_rotation: BlockRotation) {
        self.set_block_at_from_id(coords, b.id(), block_rotation);
    }

    fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_rotation: BlockRotation) {
        self.debug_assert_is_within_blocks(coords);

        let index = Self::flatten(coords);

        self.block_info[index].set_rotation(block_rotation);

        if self.blocks[index] != id {
            if self.blocks[index] == AIR_BLOCK_ID {
                self.non_air_blocks += 1;
            } else if id == AIR_BLOCK_ID {
                self.non_air_blocks -= 1;
            }

            self.blocks[index] = id;
        }
    }

    fn block_at(&self, coords: ChunkBlockCoordinate) -> u16 {
        self.blocks[Self::flatten(coords)]
    }

    fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockRotation {
        self.block_info[Self::flatten(coords)].get_rotation()
    }

    fn is_empty(&self) -> bool {
        self.non_air_blocks == 0
    }

    fn blocks(&self) -> Iter<u16> {
        self.blocks.iter()
    }

    fn block_info_iterator(&self) -> Iter<BlockInfo> {
        self.block_info.iter()
    }

    fn block_info_at(&self, coords: ChunkBlockCoordinate) -> BlockInfo {
        self.block_info[Self::flatten(coords)]
    }

    fn set_block_info_at(&mut self, coords: ChunkBlockCoordinate, block_info: BlockInfo) {
        self.block_info[Self::flatten(coords)] = block_info;
    }

    #[inline]
    fn has_full_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        blocks.from_numeric_id(self.block_at(coords)).is_full()
    }

    #[inline]
    fn has_see_through_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        blocks.from_numeric_id(self.block_at(coords)).is_see_through()
    }

    #[inline]
    fn has_block_at(&self, coords: ChunkBlockCoordinate) -> bool {
        self.is_within_blocks(coords) && self.block_at(coords) != AIR_BLOCK_ID
    }

    #[inline]
    fn is_within_blocks(&self, coords: ChunkBlockCoordinate) -> bool {
        coords.x < self.width && coords.y < self.height && coords.z < self.length
    }
}
