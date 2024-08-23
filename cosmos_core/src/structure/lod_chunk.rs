//! Used to store the various blocks an Lod would be made of

use std::fmt::Debug;

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    block::{block_rotation::BlockRotation, Block},
    registry::Registry,
    utils::array_utils::flatten,
};

use super::{
    block_storage::{BlockStorage, BlockStorer},
    chunk::{BlockInfo, CHUNK_DIMENSIONS},
    coordinates::ChunkBlockCoordinate,
};

#[derive(Reflect, Serialize, Deserialize, Clone, PartialEq)]
/// A chunk that is scaled. The Lod's scale depends on the position in the octree and size of its structure.
///
/// Lods only function properly on structures whos sizes are powers of two.
pub struct LodChunk(BlockStorage, Vec<LodBlockSubScale>);

#[derive(Reflect, Serialize, Deserialize, Clone, PartialEq, Copy, Debug)]
/// Represents how this block should be scaled - relative to the normal LOD scaling.
///
/// This is for sea-level blocks that should appear to be flat, even though they would normally get taller as the LOD
/// scaling level increases.
///
/// ## TODO:
/// **This struct is way too big (memory-wise) and should be shrunk at some point.**
/// Potential optimizations:
///   - The scaling_x/y/z variables are all <= 1.0, and f32 precision is definitely overkill.
///   - Perhaps look into f16?
pub struct LodBlockSubScale {
    /// How much the scaling should be adjusted for X coordinates in the model's positions
    pub scaling_x: f32,
    /// How much X coordinates in the model's vertices should be offset by.
    ///
    /// This is to compensate for the decrease in scaling by lowering the block to not be hovering.
    pub x_offset: f32,
    /// How much the scaling should be adjusted for Y coordinates in the model's positions
    pub scaling_y: f32,
    /// How much Y coordinates in the model's vertices should be offset by.
    ///
    /// This is to compensate for the decrease in scaling by lowering the block to not be hovering.
    pub y_offset: f32,
    /// How much the scaling should be adjusted for Z coordinates in the model's positions
    pub scaling_z: f32,
    /// How much Z coordinates in the model's vertices should be offset by.
    ///
    /// This is to compensate for the decrease in scaling by lowering the block to not be hovering.
    pub z_offset: f32,
}

impl Default for LodBlockSubScale {
    fn default() -> Self {
        Self {
            scaling_x: 1.0,
            scaling_y: 1.0,
            scaling_z: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
        }
    }
}

impl Debug for LodChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("LodChunk (empty: {})", self.0.is_empty()))
    }
}

impl Default for LodChunk {
    fn default() -> Self {
        Self::new()
    }
}

impl LodChunk {
    /// Creates a new Lod chunk
    pub fn new() -> Self {
        Self(
            BlockStorage::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS),
            vec![LodBlockSubScale::default(); (CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS) as usize],
        )
    }

    /// Sets the sub-scaling of this block within the LOD.
    pub fn set_block_scale_at(&mut self, coords: ChunkBlockCoordinate, scale: LodBlockSubScale) {
        self.1[flatten(
            coords.x as usize,
            coords.y as usize,
            coords.z as usize,
            CHUNK_DIMENSIONS as usize,
            CHUNK_DIMENSIONS as usize,
        )] = scale;
    }

    /// Gets the sub-scaling of this block within the LOD.
    pub fn block_scale(&self, coords: ChunkBlockCoordinate) -> LodBlockSubScale {
        self.1[flatten(
            coords.x as usize,
            coords.y as usize,
            coords.z as usize,
            CHUNK_DIMENSIONS as usize,
            CHUNK_DIMENSIONS as usize,
        )]
    }
}

impl BlockStorer for LodChunk {
    #[inline(always)]
    fn block_at(&self, coords: ChunkBlockCoordinate) -> u16 {
        self.0.block_at(coords)
    }

    #[inline(always)]
    fn block_info_iterator(&self) -> std::slice::Iter<BlockInfo> {
        self.0.block_info_iterator()
    }

    fn block_info_at(&self, coords: ChunkBlockCoordinate) -> BlockInfo {
        self.0.block_info_at(coords)
    }

    fn set_block_info_at(&mut self, coords: ChunkBlockCoordinate, block_info: BlockInfo) {
        self.0.set_block_info_at(coords, block_info);
    }

    #[inline(always)]
    fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockRotation {
        self.0.block_rotation(coords)
    }

    #[inline(always)]
    fn blocks(&self) -> std::slice::Iter<u16> {
        self.0.blocks()
    }

    #[inline(always)]
    fn debug_assert_is_within_blocks(&self, coords: ChunkBlockCoordinate) {
        self.0.debug_assert_is_within_blocks(coords)
    }

    #[inline(always)]
    fn has_block_at(&self, coords: ChunkBlockCoordinate) -> bool {
        self.0.has_block_at(coords)
    }

    #[inline(always)]
    fn has_full_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        self.0.has_full_block_at(coords, blocks)
    }

    #[inline(always)]
    fn has_see_through_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        self.0.has_see_through_block_at(coords, blocks)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline(always)]
    fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_up: BlockRotation) {
        self.0.set_block_at(coords, b, block_up)
    }

    #[inline(always)]
    fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_up: BlockRotation) {
        self.0.set_block_at_from_id(coords, id, block_up)
    }

    #[inline(always)]
    fn is_within_blocks(&self, coords: ChunkBlockCoordinate) -> bool {
        self.0.is_within_blocks(coords)
    }
}
