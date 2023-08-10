//! Contains all the functionality & information related to structures that are dynamically loaded.
//!
//! This means that not all chunks will be loaded at a time, and they will be loaded & unloaded at will

use bevy::{
    prelude::{Commands, Entity, EventWriter, GlobalTransform, Vec3},
    reflect::Reflect,
    utils::{hashbrown::HashSet, HashMap},
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{blocks::AIR_BLOCK_ID, hardness::BlockHardness, Block, BlockFace},
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
};

use super::{
    base_structure::BaseStructure,
    block_health::block_destroyed_event::BlockDestroyedEvent,
    chunk::{Chunk, ChunkUnloadEvent, CHUNK_DIMENSIONS},
    coordinates::{
        BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, Coordinate, CoordinateType, UnboundBlockCoordinate, UnboundChunkCoordinate,
    },
    structure_block::StructureBlock,
    structure_iterator::{BlockIterator, ChunkIterator},
    ChunkState, Structure,
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
    unloaded_chunk_blocks: HashMap<ChunkCoordinate, HashMap<ChunkBlockCoordinate, (u16, BlockFace)>>,

    dimensions: CoordinateType,
}

impl DynamicStructure {
    pub fn new(dimensions: CoordinateType) -> Self {
        Self {
            base_structure: BaseStructure::new(ChunkCoordinate::new(dimensions, dimensions, dimensions)),
            empty_chunks: HashSet::default(),
            loading_chunks: HashSet::default(),
            unloaded_chunk_blocks: HashMap::default(),
            dimensions,
        }
    }

    pub fn dimensions(&self) -> CoordinateType {
        self.dimensions
    }

    pub fn block_dimensions(&self) -> CoordinateType {
        self.dimensions * CHUNK_DIMENSIONS
    }

    #[inline(always)]
    pub(super) fn flatten(&self, c: ChunkCoordinate) -> usize {
        c.flatten(self.dimensions(), self.dimensions())
    }

    /// Returns if the chunk at these chunk coordinates is fully loaded & empty.
    pub fn has_empty_chunk_at(&self, coords: ChunkCoordinate) -> bool {
        self.get_chunk_state(coords) == ChunkState::Loaded && self.empty_chunks.contains(&self.flatten(coords))
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

        let mut send_event = true;
        if let Some(chunk) = self.mut_chunk_from_chunk_coordinates(chunk_coords) {
            chunk.set_block_at(chunk_block_coords, block, block_up);

            if chunk.is_empty() {
                self.base_structure.unload_chunk(chunk_coords);
            }
        } else if block.id() != AIR_BLOCK_ID {
            if self.get_chunk_state(chunk_coords) == ChunkState::Loaded {
                let chunk = self.create_chunk_at(chunk_coords);
                chunk.set_block_at(chunk_block_coords, block, block_up);
            } else {
                // put into some chunk queue that will be put into the chunk once it's loaded
                if !self.unloaded_chunk_blocks.contains_key(&chunk_coords) {
                    self.unloaded_chunk_blocks.insert(chunk_coords, HashMap::new());
                }
                self.unloaded_chunk_blocks
                    .get_mut(&chunk_coords)
                    .expect("Chunk hashmap insert above failed")
                    .insert(chunk_block_coords, (block.id(), block_up));

                send_event = false;
            }
        }

        if send_event {
            if let Some(self_entity) = self.get_entity() {
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
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.set_block_at(coords, blocks.from_numeric_id(AIR_BLOCK_ID), BlockFace::Top, blocks, event_writer);
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
        Self::block_relative_position_static(coords, self.dimensions())
    }

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

    pub fn chunk_entity(&self, coords: ChunkCoordinate) -> Option<bevy::prelude::Entity> {
        self.base_structure.chunk_entity(coords)
    }

    pub fn set_chunk_entity(&mut self, coords: ChunkCoordinate, entity: bevy::prelude::Entity) {
        self.base_structure.set_chunk_entity(coords, entity)
    }

    pub fn chunk_coords_within(&self, coords: ChunkCoordinate) -> bool {
        self.base_structure.chunk_coords_within(coords)
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
        event_writer: Option<&mut EventWriter<ChunkUnloadEvent>>,
    ) -> Option<Chunk> {
        let index = self.flatten(coords);

        self.empty_chunks.remove(&index);
        let chunk = self.base_structure.chunks.remove(&index);

        if let Some(entity) = self.base_structure.chunk_entities.remove(&index) {
            if let Some(event_writer) = event_writer {
                event_writer.send(ChunkUnloadEvent {
                    chunk_entity: entity,
                    coords,
                    structure_entity: self.get_entity().expect("A structure should have an entity at this point"),
                });
            }
            commands.entity(entity).insert(NeedsDespawned);
        }

        chunk
    }

    pub fn chunk_from_entity(&self, entity: &Entity) -> Option<&Chunk> {
        self.base_structure.chunk_from_entity(entity)
    }

    pub fn set_entity(&mut self, entity: bevy::prelude::Entity) {
        self.base_structure.set_entity(entity)
    }

    pub fn get_entity(&self) -> Option<Entity> {
        self.base_structure.get_entity()
    }

    pub fn chunk_from_chunk_coordinates(&self, coords: ChunkCoordinate) -> Option<&Chunk> {
        self.base_structure.chunk_from_chunk_coordinates(coords)
    }

    pub fn chunk_from_chunk_coordinates_unbound(&self, unbound_coords: UnboundChunkCoordinate) -> Option<&Chunk> {
        self.base_structure.chunk_from_chunk_coordinates_unbound(unbound_coords)
    }

    pub fn mut_chunk_from_chunk_coordinates(&mut self, coords: ChunkCoordinate) -> Option<&mut Chunk> {
        self.base_structure.mut_chunk_from_chunk_coordinates(coords)
    }

    pub fn chunk_at_block_coordinates(&self, coords: BlockCoordinate) -> Option<&Chunk> {
        self.base_structure.chunk_at_block_coordinates(coords)
    }

    pub fn is_within_blocks(&self, coords: BlockCoordinate) -> bool {
        self.base_structure.is_within_blocks(coords)
    }

    pub fn has_block_at(&self, coords: BlockCoordinate) -> bool {
        self.base_structure.has_block_at(coords)
    }

    pub fn relative_coords_to_local_coords_checked(&self, x: f32, y: f32, z: f32) -> Result<BlockCoordinate, bool> {
        self.base_structure.relative_coords_to_local_coords_checked(x, y, z)
    }

    pub fn relative_coords_to_local_coords(&self, x: f32, y: f32, z: f32) -> UnboundBlockCoordinate {
        self.base_structure.relative_coords_to_local_coords(x, y, z)
    }

    pub fn block_rotation(&self, coords: BlockCoordinate) -> BlockFace {
        self.base_structure.block_rotation(coords)
    }

    pub fn block_id_at(&self, coords: BlockCoordinate) -> u16 {
        self.base_structure.block_id_at(coords)
    }

    pub fn block_at<'a>(&'a self, coords: BlockCoordinate, blocks: &'a Registry<Block>) -> &'a Block {
        self.base_structure.block_at(coords, blocks)
    }

    pub fn chunks(&self) -> &bevy::utils::hashbrown::HashMap<usize, Chunk> {
        self.base_structure.chunks()
    }

    pub fn chunk_relative_position(&self, coords: ChunkCoordinate) -> Vec3 {
        self.base_structure.chunk_relative_position(coords)
    }

    pub fn block_world_location(&self, coords: BlockCoordinate, body_position: &GlobalTransform, this_location: &Location) -> Location {
        self.base_structure.block_world_location(coords, body_position, this_location)
    }

    pub fn take_chunk(&mut self, coords: ChunkCoordinate) -> Option<Chunk> {
        self.base_structure.take_chunk(coords)
    }

    pub fn all_chunks_iter<'a>(&'a self, structure: &'a Structure, include_empty: bool) -> ChunkIterator {
        self.base_structure.all_chunks_iter(structure, include_empty)
    }

    pub fn chunk_iter<'a>(
        &'a self,
        structure: &'a Structure,
        start: UnboundChunkCoordinate,
        end: UnboundChunkCoordinate,
        include_empty: bool,
    ) -> ChunkIterator {
        self.base_structure.chunk_iter(structure, start, end, include_empty)
    }

    pub fn block_iter_for_chunk<'a>(&'a self, structure: &'a Structure, coords: ChunkCoordinate, include_air: bool) -> BlockIterator {
        self.base_structure.block_iter_for_chunk(structure, coords, include_air)
    }

    pub fn all_blocks_iter<'a>(&'a self, structure: &'a Structure, include_air: bool) -> BlockIterator {
        self.base_structure.all_blocks_iter(structure, include_air)
    }

    pub fn block_iter<'a>(
        &'a self,
        structure: &'a Structure,
        start: UnboundBlockCoordinate,
        end: UnboundBlockCoordinate,
        include_air: bool,
    ) -> BlockIterator {
        self.base_structure.block_iter(structure, start, end, include_air)
    }

    pub fn get_block_health(&self, coords: BlockCoordinate, block_hardness: &crate::block::hardness::BlockHardness) -> f32 {
        self.base_structure.get_block_health(coords, block_hardness)
    }

    pub fn block_take_damage(
        &mut self,
        coords: BlockCoordinate,
        block_hardness: &BlockHardness,
        amount: f32,
        event_writer: Option<&mut EventWriter<BlockDestroyedEvent>>,
    ) -> bool {
        self.base_structure.block_take_damage(coords, block_hardness, amount, event_writer)
    }

    pub fn remove_chunk_entity(&mut self, coords: ChunkCoordinate) {
        self.base_structure.remove_chunk_entity(coords)
    }
}
