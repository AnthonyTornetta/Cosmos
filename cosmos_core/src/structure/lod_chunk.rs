use std::slice::Iter;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockFace},
    registry::Registry,
};

use super::{
    block_storage::BlockStorage,
    chunk::{BlockInfo, CHUNK_DIMENSIONS},
    coordinates::{ChunkBlockCoordinate, CoordinateType},
};

#[derive(Debug, Reflect, Serialize, Deserialize)]
pub struct LodChunk {
    // N chunks this contains = scale^2
    scale: CoordinateType,
    block_storage: BlockStorage,
}

impl LodChunk {
    pub fn new(scale: CoordinateType) -> Self {
        Self {
            scale,
            block_storage: BlockStorage::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS),
        }
    }

    #[inline]
    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    pub fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_up: BlockFace) {
        self.block_storage.set_block_at(coords, b, block_up);
    }

    /// Sets the block at the given location.
    ///
    /// Generally, you should use the structure's version of this because this doesn't handle everything the structure does.
    /// You should only call this if you know what you're doing.
    ///
    /// No events are generated from this.
    pub fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_up: BlockFace) {
        self.block_storage.set_block_at_from_id(coords, id, block_up);
    }

    #[inline]
    /// Gets the block at this location. Air is returned for empty blocks.
    pub fn block_at(&self, coords: ChunkBlockCoordinate) -> u16 {
        self.block_storage.block_at(coords)
    }

    #[inline]
    /// Gets the block's rotation at this location
    pub fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockFace {
        self.block_storage.block_rotation(coords)
    }

    #[inline]
    /// Returns true if this chunk only contains air.
    pub fn is_empty(&self) -> bool {
        self.block_storage.is_empty()
    }

    /// Returns the iterator for every block in the chunk
    pub fn blocks(&self) -> Iter<u16> {
        self.block_storage.blocks()
    }

    /// Returns the iterator for all the block info of the chunk
    pub fn block_info_iterator(&self) -> Iter<BlockInfo> {
        self.block_storage.block_info_iterator()
    }

    #[inline]
    /// Returns true if the block at these coordinates is a full block (1x1x1 cube). This is not determined
    /// by the model, but rather the flags the block is constructed with.
    pub fn has_full_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        self.block_storage.has_full_block_at(coords, blocks)
    }

    #[inline]
    /// Returns true if the block at this location is see-through. This is not determined from the block's texture, but
    /// rather the flags the block was constructed with.
    pub fn has_see_through_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        self.block_storage.has_see_through_block_at(coords, blocks)
    }

    #[inline]
    /// Returns true if the block at this location is not air.
    pub fn has_block_at(&self, coords: ChunkBlockCoordinate) -> bool {
        self.block_storage.has_block_at(coords)
    }
}
