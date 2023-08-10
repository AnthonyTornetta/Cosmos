//! Contains all the functionality & information related to structures that are fully loaded at all times.
//!
//! This means that all chunks this structure needs are loaded as long as the structure exists.

use bevy::{
    prelude::{Deref, DerefMut, EventWriter, Vec3},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{blocks::AIR_BLOCK_ID, Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
};

use super::{
    base_structure::BaseStructure,
    coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
    structure_block::StructureBlock,
    ChunkState,
};

#[derive(Serialize, Deserialize, Reflect, Debug, DerefMut, Deref)]
/// Contains all the functionality & information related to structures that are fully loaded at all times.
///
/// This means that all chunks this structure needs are loaded as long as the structure exists.
pub struct FullStructure {
    #[deref]
    base_structure: BaseStructure,
    loaded: bool,
}

impl FullStructure {
    pub fn new(dimensions: ChunkCoordinate) -> Self {
        Self {
            base_structure: BaseStructure::new(dimensions),
            loaded: false,
        }
    }

    /// A static version of [`Self::block_relative_position`]. This is useful if you know
    /// the dimensions of the structure, but don't have access to the structure instance.
    ///
    /// Gets the block's relative position to any structure's transform.
    ///
    /// The width, height, and length should be that structure's width, height, and length.
    pub fn block_relative_position_static(
        coords: BlockCoordinate,
        structure_blocks_width: CoordinateType,
        structure_blocks_height: CoordinateType,
        structure_blocks_length: CoordinateType,
    ) -> Vec3 {
        let xoff = structure_blocks_width as f32 / 2.0;
        let yoff = structure_blocks_height as f32 / 2.0;
        let zoff = structure_blocks_length as f32 / 2.0;

        let xx = coords.x as f32 - xoff;
        let yy = coords.y as f32 - yoff;
        let zz = coords.z as f32 - zoff;

        Vec3::new(xx + 0.5, yy + 0.5, zz + 0.5)
    }

    /// Gets the block's relative position to this structure's transform.
    pub fn block_relative_position(&self, coords: BlockCoordinate) -> Vec3 {
        Self::block_relative_position_static(coords, self.blocks_width(), self.blocks_height(), self.blocks_length())
    }

    /// Sets the block at the given block coordinates.
    ///
    /// * `event_writer` If this is `None`, no event will be generated. A valid usecase for this being `None` is when you are initially loading/generating everything and you don't want a billion events being generated.
    pub fn set_block_at(
        &mut self,
        coords: BlockCoordinate,
        block: &Block,
        block_up: BlockFace,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        let old_block = self.block_id_at(coords);
        if blocks.from_numeric_id(old_block) == block {
            return;
        }

        let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
        let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);

        if let Some(chunk) = self.mut_chunk_from_chunk_coordinates(chunk_coords) {
            chunk.set_block_at(chunk_block_coords, block, block_up);

            if chunk.is_empty() {
                self.unload_chunk(chunk_coords);
            }

            if let Some(self_entity) = self.self_entity {
                if let Some(event_writer) = event_writer {
                    event_writer.send(BlockChangedEvent {
                        new_block: block.id(),
                        old_block,
                        structure_entity: self_entity,
                        block: StructureBlock::new(coords),
                        old_block_up: self.block_rotation(coords),
                        new_block_up: block_up,
                    });
                }
            }
        }
    }

    /// Removes the block at the given coordinates
    ///
    /// * `event_writer` If this is None, no event will be generated.
    pub fn remove_block_at(
        &mut self,
        coords: BlockCoordinate,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.set_block_at(coords, blocks.from_numeric_id(AIR_BLOCK_ID), BlockFace::Top, blocks, event_writer);
    }

    /// Marks this structure as being completely loaded
    pub fn set_loaded(&mut self) {
        self.loaded = true;
    }

    /// Returns the chunk's state
    pub fn get_chunk_state(&self, coords: ChunkCoordinate) -> ChunkState {
        if !self.is_within_chunks(coords) {
            ChunkState::Invalid
        } else if self.loaded {
            ChunkState::Loaded
        } else {
            ChunkState::Loading
        }
    }

    fn is_within_chunks(&self, coords: ChunkCoordinate) -> bool {
        let (w, h, l) = self.block_dimensions().into();

        coords.x < w && coords.y < h && coords.z < l
    }
}
