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
    pub fn dimensions(&self) -> CoordinateType {
        self.dimensions
    }

    /// The number of blocks in each x/y/z axis.
    #[inline(always)]
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

    /// Returns the entity for this chunk -- an empty chunk WILL NOT have an entity.
    ///
    /// If this returns none, that means the chunk entity was not set before being used.
    #[inline(always)]
    pub fn chunk_entity(&self, coords: ChunkCoordinate) -> Option<Entity> {
        self.base_structure.chunk_entity(coords)
    }

    /// Sets the entity for the chunk at those chunk coordinates.
    ///
    /// This should be handled automatically, so you shouldn't have to call this unless
    /// you're doing some crazy stuff.
    #[inline(always)]
    pub fn set_chunk_entity(&mut self, coords: ChunkCoordinate, entity: Entity) {
        self.base_structure.set_chunk_entity(coords, entity)
    }

    /// Returns true if these chunk coordinates are within the structure
    #[inline(always)]
    pub fn chunk_coords_within(&self, coords: ChunkCoordinate) -> bool {
        self.base_structure.chunk_coords_within(coords)
    }

    /// Gets the chunk associated with the provided entity.
    #[inline(always)]
    pub fn chunk_from_entity(&self, entity: &Entity) -> Option<&Chunk> {
        self.base_structure.chunk_from_entity(entity)
    }

    /// Associates an entity with this structure.
    #[inline(always)]
    pub fn set_entity(&mut self, entity: bevy::prelude::Entity) {
        self.base_structure.set_entity(entity)
    }

    /// Retrieves the entity associated with this structure.
    #[inline(always)]
    pub fn get_entity(&self) -> Option<Entity> {
        self.base_structure.get_entity()
    }

    /// Gets the chunk associated with the provided chunk coordinates.
    #[inline(always)]
    pub fn chunk_from_chunk_coordinates(&self, coords: ChunkCoordinate) -> Option<&Chunk> {
        self.base_structure.chunk_from_chunk_coordinates(coords)
    }

    /// Gets the chunk associated with the provided unbound chunk coordinates.
    #[inline(always)]
    pub fn chunk_from_chunk_coordinates_unbound(&self, unbound_coords: UnboundChunkCoordinate) -> Option<&Chunk> {
        self.base_structure.chunk_from_chunk_coordinates_unbound(unbound_coords)
    }

    /// Gets a mutable reference to the chunk associated with the provided chunk coordinates.
    #[inline(always)]
    pub fn mut_chunk_from_chunk_coordinates(&mut self, coords: ChunkCoordinate) -> Option<&mut Chunk> {
        self.base_structure.mut_chunk_from_chunk_coordinates(coords)
    }

    /// Gets the chunk at the block coordinates.
    #[inline(always)]
    pub fn chunk_at_block_coordinates(&self, coords: BlockCoordinate) -> Option<&Chunk> {
        self.base_structure.chunk_at_block_coordinates(coords)
    }

    /// Checks if the given block coordinates are within the structure's bounds.
    #[inline(always)]
    pub fn is_within_blocks(&self, coords: BlockCoordinate) -> bool {
        self.base_structure.is_within_blocks(coords)
    }

    /// Checks if a non-air block exists at the provided coordinates.
    #[inline(always)]
    pub fn has_block_at(&self, coords: BlockCoordinate) -> bool {
        self.base_structure.has_block_at(coords)
    }

    /// Converts relative coordinates to local coordinates and performs bounds checking.
    #[inline(always)]
    pub fn relative_coords_to_local_coords_checked(&self, x: f32, y: f32, z: f32) -> Result<BlockCoordinate, bool> {
        self.base_structure.relative_coords_to_local_coords_checked(x, y, z)
    }

    /// Converts relative coordinates to local coordinates.
    #[inline(always)]
    pub fn relative_coords_to_local_coords(&self, x: f32, y: f32, z: f32) -> UnboundBlockCoordinate {
        self.base_structure.relative_coords_to_local_coords(x, y, z)
    }

    /// Retrieves the rotation of the block at the provided coordinates.
    #[inline(always)]
    pub fn block_rotation(&self, coords: BlockCoordinate) -> BlockFace {
        self.base_structure.block_rotation(coords)
    }

    /// Retrieves the block ID at the provided block coordinates.
    #[inline(always)]
    pub fn block_id_at(&self, coords: BlockCoordinate) -> u16 {
        self.base_structure.block_id_at(coords)
    }

    /// Retrieves the block at the provided block coordinates.
    #[inline(always)]
    pub fn block_at<'a>(&'a self, coords: BlockCoordinate, blocks: &'a Registry<Block>) -> &'a Block {
        self.base_structure.block_at(coords, blocks)
    }

    /// Retrieves the loaded chunks in the structure.
    #[inline(always)]
    pub fn chunks(&self) -> &bevy::utils::hashbrown::HashMap<usize, Chunk> {
        self.base_structure.chunks()
    }

    /// Retrieves the relative position of a chunk within the structure.
    #[inline(always)]
    pub fn chunk_relative_position(&self, coords: ChunkCoordinate) -> Vec3 {
        self.base_structure.chunk_relative_position(coords)
    }

    /// Calculates the world location of a block.
    #[inline(always)]
    pub fn block_world_location(&self, coords: BlockCoordinate, body_position: &GlobalTransform, this_location: &Location) -> Location {
        self.base_structure.block_world_location(coords, body_position, this_location)
    }

    /// Takes ownership of the chunk at the given chunk coordinates.
    #[inline(always)]
    pub fn take_chunk(&mut self, coords: ChunkCoordinate) -> Option<Chunk> {
        self.base_structure.take_chunk(coords)
    }

    /// Iterates over all chunks within the structure.
    #[inline(always)]
    pub fn all_chunks_iter<'a>(&'a self, structure: &'a Structure, include_empty: bool) -> ChunkIterator {
        self.base_structure.all_chunks_iter(structure, include_empty)
    }

    /// Iterates over chunks within the specified range.
    #[inline(always)]
    pub fn chunk_iter<'a>(
        &'a self,
        structure: &'a Structure,
        start: UnboundChunkCoordinate,
        end: UnboundChunkCoordinate,
        include_empty: bool,
    ) -> ChunkIterator {
        self.base_structure.chunk_iter(structure, start, end, include_empty)
    }

    /// Iterates over blocks within a specific chunk.
    #[inline(always)]
    pub fn block_iter_for_chunk<'a>(&'a self, structure: &'a Structure, coords: ChunkCoordinate, include_air: bool) -> BlockIterator {
        self.base_structure.block_iter_for_chunk(structure, coords, include_air)
    }

    /// Iterates over all blocks within the structure.
    #[inline(always)]
    pub fn all_blocks_iter<'a>(&'a self, structure: &'a Structure, include_air: bool) -> BlockIterator {
        self.base_structure.all_blocks_iter(structure, include_air)
    }

    /// Iterates over blocks within the specified range.
    #[inline(always)]
    pub fn block_iter<'a>(
        &'a self,
        structure: &'a Structure,
        start: UnboundBlockCoordinate,
        end: UnboundBlockCoordinate,
        include_air: bool,
    ) -> BlockIterator {
        self.base_structure.block_iter(structure, start, end, include_air)
    }

    /// Retrieves the health of the block at the provided coordinates.
    #[inline(always)]
    pub fn get_block_health(&self, coords: BlockCoordinate, block_hardness: &crate::block::hardness::BlockHardness) -> f32 {
        self.base_structure.get_block_health(coords, block_hardness)
    }

    /// Causes a block at the given coordinates to take damage.
    ///
    /// Returns true if the block was destroyed, false if not.
    #[inline(always)]
    pub fn block_take_damage(
        &mut self,
        coords: BlockCoordinate,
        block_hardness: &BlockHardness,
        amount: f32,
        event_writer: Option<&mut EventWriter<BlockDestroyedEvent>>,
    ) -> bool {
        self.base_structure.block_take_damage(coords, block_hardness, amount, event_writer)
    }

    /// Removes the chunk entity at the specified chunk coordinates.
    #[inline(always)]
    pub fn remove_chunk_entity(&mut self, coords: ChunkCoordinate) {
        self.base_structure.remove_chunk_entity(coords)
    }
}
