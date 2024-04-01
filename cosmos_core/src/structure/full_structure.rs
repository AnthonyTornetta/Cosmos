//! Contains all the functionality & information related to structures that are fully loaded at all times.
//!
//! This means that all chunks this structure needs are loaded as long as the structure exists.

use std::ops::{Deref, DerefMut};

use bevy::{
    prelude::{EventWriter, Vec3},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{blocks::AIR_BLOCK_ID, Block, BlockRotation},
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
};

use super::{
    base_structure::BaseStructure,
    block_storage::BlockStorer,
    chunk::Chunk,
    coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
    structure_block::StructureBlock,
    ChunkState,
};

#[derive(Serialize, Deserialize, Reflect, Debug)]
/// Contains all the functionality & information related to structures that are fully loaded at all times.
///
/// This means that all chunks this structure needs are loaded as long as the structure exists.
pub struct FullStructure {
    base_structure: BaseStructure,
    #[serde(skip)]
    loaded: bool,
}

impl Deref for FullStructure {
    type Target = BaseStructure;

    fn deref(&self) -> &Self::Target {
        &self.base_structure
    }
}

impl DerefMut for FullStructure {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_structure
    }
}

impl FullStructure {
    /// A full structure has all its chunks loaded at the same time.
    ///
    /// - `dimensions` The x/y/z dimensions of the structure. These do not have to be the same
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
        block_rotation: BlockRotation,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.base_structure.debug_assert_block_coords_within(coords);

        let old_block = self.block_id_at(coords);
        if blocks.from_numeric_id(old_block) == block && self.block_rotation(coords) == block_rotation {
            return;
        }

        let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
        let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);
        let mut send_event = false;

        if let Some(chunk) = self.mut_chunk_from_chunk_coordinates(chunk_coords) {
            chunk.set_block_at(chunk_block_coords, block, block_rotation);

            if chunk.is_empty() {
                self.base_structure.unload_chunk(chunk_coords);
            }

            send_event = true;
        } else if block.id() != AIR_BLOCK_ID {
            let mut chunk = Chunk::new(chunk_coords);
            chunk.set_block_at(chunk_block_coords, block, block_rotation);
            self.base_structure.chunks.insert(self.base_structure.flatten(chunk_coords), chunk);
            send_event = true;
        }

        if !send_event {
            return;
        }
        let Some(self_entity) = self.base_structure.self_entity else {
            return;
        };
        let Some(event_writer) = event_writer else {
            return;
        };

        event_writer.send(BlockChangedEvent {
            new_block: block.id(),
            old_block,
            structure_entity: self_entity,
            block: StructureBlock::new(coords),
            old_block_rotation: self.block_rotation(coords),
            new_block_rotation: block_rotation,
        });
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
        self.set_block_at(
            coords,
            blocks.from_numeric_id(AIR_BLOCK_ID),
            BlockRotation::default(),
            blocks,
            event_writer,
        );
    }

    /// Marks this structure as being completely loaded
    pub fn set_loaded(&mut self) {
        self.loaded = true;
    }

    /// Returns true if the `set_loaded` member function has been called.
    pub fn is_loaded(&self) -> bool {
        self.loaded
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

    /// Returns if the chunk at these chunk coordinates is fully loaded & empty.
    pub fn has_empty_chunk_at(&self, coords: ChunkCoordinate) -> bool {
        self.get_chunk_state(coords) == ChunkState::Loaded
            && self.chunk_from_chunk_coordinates(coords).map(|c| c.is_empty()).unwrap_or(true)
    }
}
