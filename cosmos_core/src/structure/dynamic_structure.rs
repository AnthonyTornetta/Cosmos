//! Contains all the functionality & information related to structures that are dynamically loaded.
//!
//! This means that not all chunks will be loaded at a time, and they will be loaded & unloaded at will

use std::ops::{Deref, DerefMut};

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, block_rotation::BlockRotation, blocks::AIR_BLOCK_ID},
    ecs::NeedsDespawned,
    events::block_events::{BlockChangedMessage, BlockChangedReason},
    registry::{Registry, identifiable::Identifiable},
};

use super::{
    ChunkState,
    base_structure::BaseStructure,
    block_storage::BlockStorer,
    chunk::{BlockInfo, CHUNK_DIMENSIONS, Chunk, ChunkUnloadMessage},
    coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, Coordinate, CoordinateType},
    structure_block::StructureBlock,
};

#[derive(Serialize, Deserialize, Reflect, Debug)]
/// Contains all the functionality & information related to structures that are dynamically loaded.
///
/// This means that not all chunks will be loaded at a time, and they will be loaded & unloaded at will
pub struct DynamicStructure {
    base_structure: BaseStructure,
    /// Chunks that are just air should be removed from the chunks map above to conserve memory
    /// and added into this to be stored instead.
    empty_chunks: HashSet<usize>,

    #[serde(skip)]
    /// This does not represent every loading chunk, only those that have been
    /// specifically taken out via `take_chunk_for_loading` to be generated across multiple systems/frames.
    loading_chunks: HashSet<usize>,

    /// Outer hashmap maps coordinates of a chunk to a hashmap that maps coordinates in that chunk to block ids.
    #[serde(skip)]
    unloaded_chunk_blocks: HashMap<ChunkCoordinate, HashMap<ChunkBlockCoordinate, (u16, BlockRotation)>>,

    dimensions: CoordinateType,
}

impl Deref for DynamicStructure {
    type Target = BaseStructure;

    fn deref(&self) -> &Self::Target {
        &self.base_structure
    }
}

impl DerefMut for DynamicStructure {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_structure
    }
}

impl DynamicStructure {
    /// Creates a new dynamic structure. Note that dynamic structures only have 1 size used for all three axis.
    pub fn new(dimensions: CoordinateType) -> Self {
        Self {
            base_structure: BaseStructure::new(ChunkCoordinate::new(dimensions, dimensions, dimensions)),
            empty_chunks: HashSet::default(),
            loading_chunks: HashSet::default(),
            unloaded_chunk_blocks: HashMap::default(),
            dimensions,
        }
    }

    /// The number of chunks in each x/y/z axis
    #[inline(always)]
    pub fn chunk_dimensions(&self) -> CoordinateType {
        self.dimensions
    }

    /// The number of blocks in each x/y/z axis.
    #[inline(always)]
    pub fn block_dimensions(&self) -> CoordinateType {
        self.dimensions * CHUNK_DIMENSIONS
    }

    #[inline(always)]
    pub(super) fn flatten(&self, c: ChunkCoordinate) -> usize {
        c.flatten(self.chunk_dimensions(), self.chunk_dimensions())
    }

    /// Returns if the chunk at these chunk coordinates is fully loaded & empty.
    pub fn has_empty_chunk_at(&self, coords: ChunkCoordinate) -> bool {
        self.get_chunk_state(coords) == ChunkState::Loaded && self.empty_chunks.contains(&self.flatten(coords))
    }

    /// Sets the block at the given block coordinates.
    /// Also sets its block_info. This does NOT send a [`BlockDataChangedMessage`] event!
    ///
    /// * `event_writer` If this is `None`, no event will be generated. A valid usecase for this being `None` is when you are initially loading/generating everything and you don't want a billion events being generated.
    pub fn set_block_and_info_at(
        &mut self,
        coords: BlockCoordinate,
        block: &Block,
        block_info: BlockInfo,
        blocks: &Registry<Block>,
        event_writer: Option<(&mut MessageWriter<BlockChangedMessage>, BlockChangedReason)>,
    ) {
        let old_block = self.block_id_at(coords);
        let old_block_info = self.block_info_at(coords);

        self.set_block_at(coords, block, block_info.get_rotation(), blocks, None);
        self.set_block_info_at(coords, block_info, None);

        if let Some((event_writer, reason)) = event_writer
            && (old_block_info != block_info || old_block != block.id())
        {
            let Some(self_entity) = self.base_structure.self_entity else {
                return;
            };
            event_writer.write(BlockChangedMessage {
                new_block: block.id(),
                old_block,
                block: StructureBlock::new(coords, self_entity),
                old_block_info,
                new_block_info: self.block_info_at(coords),
                reason,
            });
        }
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
        event_writer: Option<(&mut MessageWriter<BlockChangedMessage>, BlockChangedReason)>,
    ) {
        let old_block = self.block_id_at(coords);
        if blocks.from_numeric_id(old_block) == block && self.block_rotation(coords) == block_rotation {
            return;
        }

        let old_block_info = self.block_info_at(coords);

        let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
        let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords);

        let mut send_event = true;
        if let Some(chunk) = self.mut_chunk_at(chunk_coords) {
            chunk.set_block_at(chunk_block_coords, block, block_rotation);

            if chunk.is_empty() {
                self.base_structure.unload_chunk(chunk_coords);
            }
        } else if block.id() != AIR_BLOCK_ID {
            if self.get_chunk_state(chunk_coords) == ChunkState::Loaded {
                let chunk = self.create_chunk_at(chunk_coords);
                chunk.set_block_at(chunk_block_coords, block, block_rotation);
            } else {
                // put into some chunk queue that will be put into the chunk once it's loaded
                if !self.unloaded_chunk_blocks.contains_key(&chunk_coords) {
                    self.unloaded_chunk_blocks.insert(chunk_coords, HashMap::new());
                }
                self.unloaded_chunk_blocks
                    .get_mut(&chunk_coords)
                    .expect("Chunk hashmap insert above failed")
                    .insert(chunk_block_coords, (block.id(), block_rotation));

                send_event = false;
            }
        }

        if send_event
            && let Some(self_entity) = self.get_entity()
            && let Some((event_writer, reason)) = event_writer
        {
            event_writer.write(BlockChangedMessage {
                new_block: block.id(),
                old_block,
                block: StructureBlock::new(coords, self_entity),
                old_block_info,
                new_block_info: self.block_info_at(coords),
                reason,
            });
        }
    }

    fn create_chunk_at(&mut self, coords: ChunkCoordinate) -> &mut Chunk {
        let index = self.flatten(coords);

        self.base_structure.chunks.insert(index, Chunk::new(coords));

        self.base_structure.chunks.get_mut(&index).unwrap()
    }

    /// Removes the block at the given coordinates
    ///
    /// * `event_writer` If this is None, no event will be generated.
    pub fn remove_block_at(
        &mut self,
        coords: BlockCoordinate,
        blocks: &Registry<Block>,
        event_writer: Option<(&mut MessageWriter<BlockChangedMessage>, BlockChangedReason)>,
    ) {
        self.set_block_at(
            coords,
            blocks.from_numeric_id(AIR_BLOCK_ID),
            BlockRotation::default(),
            blocks,
            event_writer,
        );
    }

    /// A static version of [`Self::block_relative_position`]. This is useful if you know
    /// the dimensions of the structure, but don't have access to the structure instance.
    ///
    /// Gets the block's relative position to any structure's transform.
    ///
    /// The dimensions should be that structure's dimensions.
    pub fn block_relative_position_static(coords: BlockCoordinate, structure_blocks_dimensions: CoordinateType) -> Vec3 {
        let off = structure_blocks_dimensions as f32 / 2.0;

        let xx = coords.x as f32 - off;
        let yy = coords.y as f32 - off;
        let zz = coords.z as f32 - off;

        Vec3::new(xx + 0.5, yy + 0.5, zz + 0.5)
    }

    /// Gets the block's relative position to this structure's transform.
    pub fn block_relative_position(&self, coords: BlockCoordinate) -> Vec3 {
        Self::block_relative_position_static(coords, self.block_dimensions())
    }

    /// Sets the chunk, overwriting what may have been there before.
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle that properly.
    ///     
    /// This will also mark the chunk as being done loading if it was loading.
    pub fn set_chunk(&mut self, mut chunk: Chunk) {
        let i = self.flatten(chunk.chunk_coordinates());

        if let Some(block_map) = self.unloaded_chunk_blocks.remove(&chunk.chunk_coordinates()) {
            for (coords, (block_id, block_up)) in block_map {
                chunk.set_block_at_from_id(coords, block_id, block_up);
            }
        }

        self.loading_chunks.remove(&i);

        if chunk.is_empty() {
            self.empty_chunks.insert(i);
        } else {
            self.empty_chunks.remove(&i);
        }

        self.base_structure.set_chunk(chunk);
    }

    /// Sets the chunk at this chunk location to be empty (all air).
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle those properly.
    ///
    /// This will also mark the chunk as being done loading if it was loading.
    pub fn set_to_empty_chunk(&mut self, coords: ChunkCoordinate) {
        self.base_structure.set_to_empty_chunk(coords);

        let i = self.flatten(coords);

        self.loading_chunks.remove(&i);
        self.empty_chunks.insert(i);
    }

    /// # ONLY CALL THIS IF YOU THEN CALL SET_CHUNK IN THE FUTURE!
    ///
    /// This takes ownership of the chunk that was at this location. Useful for
    /// multithreading stuff over multiple chunks & multiple systems + frames.
    ///
    /// If no chunk was previously at this location, this creates a new chunk for you to
    /// populate & then later insert into this structure via `set_chunk`.
    ///
    /// This will also mark the chunk as being loaded, so [`get_chunk_state`] will return
    /// `ChunkState::Loading`.
    pub fn take_or_create_chunk_for_loading(&mut self, coords: ChunkCoordinate) -> Chunk {
        self.base_structure.debug_assert_coords_within(coords);

        let idx = self.flatten(coords);
        self.loading_chunks.insert(idx);

        if let Some(c) = self.base_structure.chunks.remove(&idx) {
            c
        } else {
            self.empty_chunks.insert(idx);

            Chunk::new(coords)
        }
    }

    /// Marks a chunk as being loaded, useful for planet generation
    pub fn mark_chunk_being_loaded(&mut self, coords: ChunkCoordinate) {
        self.base_structure.debug_assert_coords_within(coords);

        let idx = self.flatten(coords);
        self.loading_chunks.insert(idx);
    }

    /// Returns the chunk's state
    pub fn get_chunk_state(&self, coords: ChunkCoordinate) -> ChunkState {
        if !self.chunk_coords_within(coords) {
            return ChunkState::Invalid;
        }

        let idx = self.flatten(coords);

        if self.loading_chunks.contains(&idx) {
            ChunkState::Loading
        } else if self.base_structure.chunks.contains_key(&idx) {
            if self.chunk_entity(coords).is_some() {
                ChunkState::Loaded
            } else {
                ChunkState::Loading
            }
        } else if self.empty_chunks.contains(&idx) {
            ChunkState::Loaded
        } else {
            ChunkState::Unloaded
        }
    }

    /// Unloads the chunk at the given chunk position
    pub fn unload_chunk_at(
        &mut self,
        coords: ChunkCoordinate,
        commands: &mut Commands,
        event_writer: Option<&mut MessageWriter<ChunkUnloadMessage>>,
    ) -> Option<Chunk> {
        let index = self.flatten(coords);

        self.empty_chunks.remove(&index);
        let chunk = self.base_structure.chunks.remove(&index);

        if let Some(entity) = self.base_structure.chunk_entities.remove(&index) {
            if let Some(event_writer) = event_writer {
                event_writer.write(ChunkUnloadMessage {
                    chunk_entity: entity,
                    coords,
                    structure_entity: self.get_entity().expect("A structure should have an entity at this point"),
                });
            }
            commands.entity(entity).insert(NeedsDespawned);
        }

        chunk
    }
}
