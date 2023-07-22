//! Contains all the functionality & information related to structures.
//!
//! Structures are the backbone of everything that contains blocks.

use std::fmt::Display;

use bevy::prelude::{App, Event, IntoSystemConfigs, PreUpdate};
use bevy::reflect::Reflect;
use bevy::utils::{HashMap, HashSet};
use bevy_rapier3d::prelude::PhysicsWorld;

pub mod asteroid;
pub mod block_health;
pub mod chunk;
pub mod coordinates;
pub mod events;
pub mod loading;
pub mod planet;
pub mod ship;
pub mod structure_block;
pub mod structure_builder;
pub mod structure_iterator;
pub mod systems;

use crate::block::blocks::AIR_BLOCK_ID;
use crate::block::hardness::BlockHardness;
use crate::block::{Block, BlockFace};
use crate::ecs::NeedsDespawned;
use crate::events::block_events::BlockChangedEvent;
use crate::netty::NoSendEntity;
use crate::physics::location::Location;
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::utils::array_utils::flatten;
use bevy::prelude::{
    BuildChildren, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform, PbrBundle, Query, States, Transform, Vec3,
};
use serde::{Deserialize, Serialize};

use self::block_health::block_destroyed_event::BlockDestroyedEvent;
use self::chunk::ChunkEntity;
use self::coordinates::{
    BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, Coordinate, CoordinateType, UnboundBlockCoordinate, UnboundChunkCoordinate,
    UnboundCoordinateType,
};
use self::events::ChunkSetEvent;
use self::structure_block::StructureBlock;
use self::structure_iterator::{BlockIterator, ChunkIterator};

#[derive(Serialize, Deserialize, Component, Reflect, Debug)]
/// A structure represents many blocks, grouped into chunks.
pub struct Structure {
    #[serde(skip)]
    chunk_entities: HashMap<usize, Entity>,
    #[serde(skip)]
    chunk_entity_map: HashMap<Entity, usize>,
    #[serde(skip)]
    self_entity: Option<Entity>,
    /// Signifies that every chunk has been loaded. This is not used
    /// on planets, but is used on ships + asteroids
    #[serde(skip)]
    chunks: HashMap<usize, Chunk>,

    /// Chunks that are just air should be removed from the chunks map above to conserve memory
    /// and added into this to be stored instead.
    empty_chunks: HashSet<usize>,

    #[serde(skip)]
    /// This does not represent every loading chunk, only those that have been
    /// specifically taken out via `take_chunk_for_loading` to be generated across multiple systems/frames.
    loading_chunks: HashSet<usize>,

    all_loaded: bool,

    /// Outer hashmap maps coordinates of a chunk to a hashmap that maps coordinates in that chunk to block ids.
    #[serde(skip)]
    unloaded_chunk_blocks: HashMap<ChunkCoordinate, HashMap<ChunkBlockCoordinate, (u16, BlockFace)>>,

    dimensions: ChunkCoordinate,
}

impl Structure {
    /// Creates a structure with a given amount of chunks.
    ///
    /// All chunks are initially unloaded, and must be manually loaded.
    ///
    /// ## Note: For planets, width, height, and length must all be equal
    ///
    /// * `width` The number of chunks in the X direction
    /// * `height` The number of chunks in the Y direction
    /// * `length` The number of chunks in the Z direction
    pub fn new(width: CoordinateType, height: CoordinateType, length: CoordinateType) -> Self {
        Self {
            chunk_entities: HashMap::default(),
            chunk_entity_map: HashMap::default(),
            self_entity: None,
            chunks: HashMap::default(),
            empty_chunks: HashSet::default(),
            loading_chunks: HashSet::default(),
            all_loaded: false,
            unloaded_chunk_blocks: HashMap::default(),
            dimensions: ChunkCoordinate::new(width, height, length),
        }
    }

    #[inline(always)]
    /// The number of chunks in the x direction
    pub fn chunks_width(&self) -> CoordinateType {
        self.dimensions.x
    }

    #[inline(always)]
    /// The number of chunks in the y direction
    pub fn chunks_height(&self) -> CoordinateType {
        self.dimensions.y
    }

    #[inline(always)]
    /// The number of chunks in the z direction
    pub fn chunks_length(&self) -> CoordinateType {
        self.dimensions.z
    }

    #[inline(always)]
    /// The number of blocks in the x direction
    pub fn blocks_width(&self) -> CoordinateType {
        self.chunks_width() * CHUNK_DIMENSIONS
    }

    #[inline(always)]
    /// The number of blocks in the y direction
    pub fn blocks_height(&self) -> CoordinateType {
        self.chunks_height() * CHUNK_DIMENSIONS
    }

    #[inline(always)]
    /// The number of blocks in the z direction
    pub fn blocks_length(&self) -> CoordinateType {
        self.chunks_length() * CHUNK_DIMENSIONS
    }

    #[inline(always)]
    fn flatten_c(&self, c: ChunkCoordinate) -> usize {
        c.flatten(self.chunks_width(), self.chunks_height())
    }

    #[inline(always)]
    fn flatten_b(&self, b: BlockCoordinate) -> usize {
        b.flatten(self.blocks_width(), self.blocks_height())
    }

    /// Returns the entity for this chunk -- an empty chunk WILL NOT have an entity.
    ///
    /// If this returns none, that means the chunk entity was not set before being used.
    /// Maybe the chunk is empty or unloaded?
    #[inline]
    pub fn chunk_entity(&self, coords: ChunkCoordinate) -> Option<Entity> {
        self.chunk_entities.get(&self.flatten_c(coords)).copied()
    }

    /// Sets the entity for the chunk at those chunk coordinates.
    ///
    /// This should be handled automatically, so you shouldn't have to call this unless
    /// you're doing some crazy stuff.
    pub fn set_chunk_entity(&mut self, coords: ChunkCoordinate, entity: Entity) {
        let index = self.flatten_c(coords);

        self.chunk_entity_map.insert(entity, index);
        self.chunk_entities.insert(index, entity);
    }

    /// Gets the chunk from its entity, or return None if there is no loaded chunk for that entity.
    ///
    /// Remember that empty chunks will NOT have an entity.
    pub fn chunk_from_entity(&self, entity: &Entity) -> Option<&Chunk> {
        self.chunk_entity_map.get(entity).map(|x| &self.chunks[x])
    }

    /// Sets this structure's entity - used in the base builder.
    pub(crate) fn set_entity(&mut self, entity: Entity) {
        self.self_entity = Some(entity);
    }

    /// Gets the structure's entity
    ///
    /// May be None if this hasn't been built yet.
    pub fn get_entity(&self) -> Option<Entity> {
        self.self_entity
    }

    /// Returns None for unloaded/empty chunks - panics for chunks that are out of bounds in debug mode
    ///  
    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_from_chunk_coordinates(&self, coords: ChunkCoordinate) -> Option<&Chunk> {
        self.debug_assert_coords_within(coords);

        self.chunks.get(&self.flatten_c(coords))
    }

    /// Returns if the chunk at these chunk coordinates is fully loaded & empty.
    pub fn has_empty_chunk_at(&self, coords: ChunkCoordinate) -> bool {
        self.get_chunk_state(coords) == ChunkState::Loaded && self.empty_chunks.contains(&self.flatten_c(coords))
    }

    /// Returns None for unloaded/empty chunks AND for chunks that are out of bounds
    ///
    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0\
    /// (-1, 0, 0) => None
    pub fn chunk_from_chunk_coordinates_unbound(&self, unbound_coords: UnboundChunkCoordinate) -> Option<&Chunk> {
        let Ok(bounded) = ChunkCoordinate::try_from(unbound_coords) else {
            return None;
        };

        if self.chunk_coords_within(bounded) {
            self.chunk_from_chunk_coordinates(bounded)
        } else {
            None
        }
    }

    /// Gets the mutable chunk for these chunk coordinates. If the chunk is unloaded OR empty, this will return None.
    ///
    /// ## Be careful with this!!
    ///
    /// Modifying a chunk will not update the structure or chunks surrounding it and it won't send any events.
    /// Unless you know what you're doing, you should use a mutable structure instead
    /// of a mutable chunk to make changes!
    pub fn mut_chunk_from_chunk_coordinates(&mut self, coords: ChunkCoordinate) -> Option<&mut Chunk> {
        self.debug_assert_coords_within(coords);

        self.chunks.get_mut(&self.flatten_c(coords))
    }

    /// Returns the chunk at those block coordinates if it is non-empty AND loaded.
    ///
    /// Ex:
    /// - (0, 0, 0) => chunk @ 0, 0, 0\
    /// - (5, 0, 0) => chunk @ 0, 0, 0\
    /// - (`CHUNK_DIMENSIONS`, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at_block_coordinates(&self, coords: BlockCoordinate) -> Option<&Chunk> {
        self.chunk_from_chunk_coordinates(ChunkCoordinate::for_block_coordinate(coords))
    }

    /// Returns the mutable chunk at those block coordinates. If the chunk is unloaded OR empty, this will return None.
    ///
    /// Ex:
    /// - (0, 0, 0) => chunk @ 0, 0, 0\
    /// - (5, 0, 0) => chunk @ 0, 0, 0\
    /// - (`CHUNK_DIMENSIONS`, 0, 0) => chunk @ 1, 0, 0
    ///
    /// ## Be careful with this!!
    /// Modifying a chunk will not update the structure or chunks surrounding it and it won't send any events.
    /// Unless you know what you're doing, you should use a mutable structure instead
    /// of a mutable chunk to make changes!
    fn mut_chunk_at_block_coordinates(&mut self, coords: BlockCoordinate) -> Option<&mut Chunk> {
        self.mut_chunk_from_chunk_coordinates(ChunkCoordinate::for_block_coordinate(coords))
    }

    /// Returns true if these block coordinates are within the structure's bounds
    ///
    /// Note that this does not guarentee that this block location is loaded.
    pub fn is_within_blocks(&self, coords: BlockCoordinate) -> bool {
        coords.x < self.blocks_width() && coords.y < self.blocks_height() && coords.z < self.blocks_length()
    }

    #[inline(always)]
    fn debug_assert_is_within_blocks(&self, coords: BlockCoordinate) {
        debug_assert!(
            coords.x < self.blocks_width() && coords.y < self.blocks_height() && coords.z < self.blocks_length(),
            "{} < {} && {} < {} && {} < {} failed",
            coords.x,
            coords.y,
            coords.z,
            self.blocks_width(),
            self.blocks_height(),
            self.blocks_length()
        );
    }

    /// Returns true if the structure has a loaded block here that isn't air.
    pub fn has_block_at(&self, coords: BlockCoordinate) -> bool {
        self.block_id_at(coords) != AIR_BLOCK_ID
    }

    /// # Arguments
    /// Coordinates relative to the structure's 0, 0, 0 position in the world mapped to block coordinates
    /// # Returns
    /// - Ok (x, y, z) of the block coordinates if the point is within the structure
    /// - Err(false) if one of the x/y/z coordinates are outside the structure in the negative direction
    /// - Err (true) if one of the x/y/z coordinates are outside the structure in the positive direction
    pub fn relative_coords_to_local_coords_checked(&self, x: f32, y: f32, z: f32) -> Result<BlockCoordinate, bool> {
        let unbound_coords = self.relative_coords_to_local_coords(x, y, z);

        if let Ok(block_coords) = BlockCoordinate::try_from(unbound_coords) {
            if self.is_within_blocks(block_coords) {
                Ok(block_coords)
            } else {
                Err(true)
            }
        } else {
            Err(false)
        }
    }

    /// # Arguments
    /// Coordinates relative to the structure's 0, 0, 0 position in the world mapped to block coordinates.
    ///
    /// These coordinates may not be within the structure (too high or negative).
    /// # Returns
    /// - (x, y, z) of the block coordinates, even if they are outside the structure
    pub fn relative_coords_to_local_coords(&self, x: f32, y: f32, z: f32) -> UnboundBlockCoordinate {
        let xx: f32 = x + (self.blocks_width() as f32 / 2.0);
        let yy = y + (self.blocks_height() as f32 / 2.0);
        let zz = z + (self.blocks_length() as f32 / 2.0);

        UnboundBlockCoordinate::new(
            xx.floor() as UnboundCoordinateType,
            yy.floor() as UnboundCoordinateType,
            zz.floor() as UnboundCoordinateType,
        )
    }

    /// Gets the block's up facing face at this location.
    ///
    /// If no block was found, returns BlockFace::Top.
    pub fn block_rotation(&self, coords: BlockCoordinate) -> BlockFace {
        self.chunk_at_block_coordinates(coords)
            .map(|chunk| chunk.block_rotation(ChunkBlockCoordinate::for_block_coordinate(coords)))
            .unwrap_or(BlockFace::Top)
    }

    /// Gets the rotation at this block coordinate tuple.
    #[deprecated = "Keeping for now because of ice planet work - will be removed soon"]
    pub fn block_rotation_tuple(&self, (x, y, z): (usize, usize, usize)) -> BlockFace {
        self.block_rotation(BlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType))
    }

    /// If the chunk is loaded, non-empty, returns the block at that coordinate.
    /// Otherwise, returns AIR_BLOCK_ID
    pub fn block_id_at(&self, coords: BlockCoordinate) -> u16 {
        self.debug_assert_is_within_blocks(coords);

        self.chunk_at_block_coordinates(coords)
            .map(|chunk| chunk.block_at(ChunkBlockCoordinate::for_block_coordinate(coords)))
            .unwrap_or(AIR_BLOCK_ID)
    }

    /// Gets the block at these block coordinates
    pub fn block_at<'a>(&'a self, coords: BlockCoordinate, blocks: &'a Registry<Block>) -> &'a Block {
        let id = self.block_id_at(coords);
        blocks.from_numeric_id(id)
    }

    /// Gets the hashmap for the loaded, non-empty chunks.
    ///
    /// This is going to be replaced with an iterator in the future
    pub fn chunks(&self) -> &HashMap<usize, Chunk> {
        &self.chunks
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

    fn create_chunk_at(&mut self, coords: ChunkCoordinate) -> &mut Chunk {
        let index = self.flatten_c(coords);

        self.chunks.insert(index, Chunk::new(coords));

        self.chunks.get_mut(&index).unwrap()
    }

    /// Removes the chunk at the given coordinate -- does NOT remove the chunk entity
    fn unload_chunk(&mut self, coords: ChunkCoordinate) {
        self.chunks.remove(&self.flatten_c(coords));
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

        // let (cx, cy, cz) = (x / CHUNK_DIMENSIONS, y / CHUNK_DIMENSIONS, z / CHUNK_DIMENSIONS);
        let chunk_coords = ChunkCoordinate::for_block_coordinate(coords);
        let chunk_block_coords = ChunkBlockCoordinate::for_block_coordinate(coords); // (bx, by, bz) = (x & (CHUNK_DIMENSIONS - 1), y & (CHUNK_DIMENSIONS - 1), z & (CHUNK_DIMENSIONS - 1));

        let mut send_event = true;
        if let Some(chunk) = self.mut_chunk_from_chunk_coordinates(chunk_coords) {
            chunk.set_block_at(chunk_block_coords, block, block_up);

            if chunk.is_empty() {
                self.unload_chunk(chunk_coords);
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

    /// Gets the chunk's relative position to this structure's transform.
    pub fn chunk_relative_position(&self, coords: ChunkCoordinate) -> Vec3 {
        let xoff = (self.chunks_width() as f32 - 1.0) / 2.0;
        let yoff = (self.chunks_height() as f32 - 1.0) / 2.0;
        let zoff = (self.chunks_length() as f32 - 1.0) / 2.0;

        let xx = CHUNK_DIMENSIONS as f32 * (coords.x as f32 - xoff);
        let yy = CHUNK_DIMENSIONS as f32 * (coords.y as f32 - yoff);
        let zz = CHUNK_DIMENSIONS as f32 * (coords.z as f32 - zoff);

        Vec3::new(xx, yy, zz)
    }

    /// Gets the block's relative position to this structure's transform.
    pub fn block_relative_position(&self, coords: BlockCoordinate) -> Vec3 {
        Self::block_relative_position_static(coords, self.blocks_width(), self.blocks_height(), self.blocks_length())
    }

    /// A static version of [`Structure::block_relative_position`]. This is useful if you know
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

    /// Gets a blocks's location in the world
    pub fn block_world_location(&self, coords: BlockCoordinate, body_position: &GlobalTransform, this_location: &Location) -> Location {
        *this_location + body_position.affine().matrix3.mul_vec3(self.block_relative_position(coords))
    }

    /// Sets the chunk, overwriting what may have been there before.
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle those properly.
    pub fn set_chunk(&mut self, mut chunk: Chunk) {
        let i = self.flatten_c(chunk.structure_coords());

        // Add blocks from hashmap.
        // chunk.set_block_at();
        if let Some(block_map) = self.unloaded_chunk_blocks.remove(&chunk.structure_coords()) {
            for (coords, (block_id, block_up)) in block_map {
                chunk.set_block_at_from_id(coords, block_id, block_up);
            }
        }

        self.loading_chunks.remove(&i);

        if chunk.is_empty() {
            self.empty_chunks.insert(i);
            self.chunks.remove(&i);
        } else {
            self.chunks.insert(i, chunk);
            self.empty_chunks.remove(&i);
        }
    }

    /// Sets the chunk at this chunk location to be empty (all air).
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle those properly.
    pub fn set_to_empty_chunk(&mut self, coords: ChunkCoordinate) {
        let i = self.flatten_c(coords);

        self.chunks.remove(&i);
        self.loading_chunks.remove(&i);
        self.empty_chunks.insert(i);
    }

    /// # ONLY CALL THIS IF YOU THEN CALL SET_CHUNK IN THE SAME SYSTEM!
    ///
    /// This takes ownership of the chunk that was at this location. Useful for
    /// multithreading stuff over multiple chunks.
    pub fn take_chunk(&mut self, coords: ChunkCoordinate) -> Option<Chunk> {
        self.debug_assert_coords_within(coords);
        self.chunks.remove(&self.flatten_c(coords))
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
        self.debug_assert_coords_within(coords);

        let idx = self.flatten_c(coords);
        self.loading_chunks.insert(idx);

        if let Some(c) = self.chunks.remove(&idx) {
            c
        } else {
            self.empty_chunks.insert(idx);

            Chunk::new(coords)
        }
    }

    /// Marks a chunk as being loaded, useful for planet generation
    pub fn mark_chunk_being_loaded(&mut self, coords: ChunkCoordinate) {
        self.debug_assert_coords_within(coords);

        let idx = self.flatten_c(coords);
        self.loading_chunks.insert(idx);
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    ///
    /// If include_empty is enabled, the value iterated over may be None OR Some(chunk).
    /// If include_empty is disabled, the value iterated over may ONLY BE Some(chunk).
    pub fn all_chunks_iter(&self, include_empty: bool) -> ChunkIterator {
        ChunkIterator::new(
            ChunkCoordinate::new(0, 0, 0).into(),
            ChunkCoordinate::new(self.chunks_width() - 1, self.chunks_height() - 1, self.chunks_length() - 1).into(),
            self,
            include_empty,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn chunk_iter(&self, start: UnboundChunkCoordinate, end: UnboundChunkCoordinate, include_empty: bool) -> ChunkIterator {
        ChunkIterator::new(start, end, self, include_empty)
    }

    /// Will fail assertion if chunk positions are out of bounds
    pub fn block_iter_for_chunk(&self, coords: ChunkCoordinate, include_air: bool) -> BlockIterator {
        self.debug_assert_coords_within(coords);

        BlockIterator::new(
            coords.first_structure_block().into(),
            coords.last_structure_block().into(),
            include_air,
            self,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn all_blocks_iter(&self, include_air: bool) -> BlockIterator {
        BlockIterator::new(
            BlockCoordinate::new(0, 0, 0).into(),
            BlockCoordinate::new(self.blocks_width() - 1, self.blocks_height() - 1, self.blocks_length() - 1).into(),
            include_air,
            self,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn block_iter(&self, start: UnboundBlockCoordinate, end: UnboundBlockCoordinate, include_air: bool) -> BlockIterator {
        BlockIterator::new(start, end, include_air, self)
    }

    /// Gets the block's health at that given coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    pub fn get_block_health(&mut self, coords: BlockCoordinate, block_hardness: &BlockHardness) -> f32 {
        self.chunk_at_block_coordinates(coords)
            .map(|c| c.get_block_health(ChunkBlockCoordinate::for_block_coordinate(coords), block_hardness))
            .unwrap_or(0.0)
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// - x/y/z: Block coordinates
    /// - block_hardness: The hardness for that block
    /// - amount: The amount of damage to take - cannot be negative
    ///
    /// Returns: true if that block was destroyed, false if not
    pub fn block_take_damage(
        &mut self,
        coords: BlockCoordinate,
        block_hardness: &BlockHardness,
        amount: f32,
        event_writer: Option<&mut EventWriter<BlockDestroyedEvent>>,
    ) -> bool {
        if let Some(chunk) = self.mut_chunk_at_block_coordinates(coords) {
            let destroyed = chunk.block_take_damage(ChunkBlockCoordinate::for_block_coordinate(coords), block_hardness, amount);

            if destroyed {
                if let Some(structure_entity) = self.get_entity() {
                    if let Some(event_writer) = event_writer {
                        event_writer.send(BlockDestroyedEvent {
                            block: StructureBlock::new(coords),
                            structure_entity,
                        });
                    }
                }
            }

            destroyed
        } else {
            false
        }
    }

    #[inline]
    /// Returns true if these chunk coordinates are within the structure
    pub fn chunk_coords_within(&self, coords: ChunkCoordinate) -> bool {
        coords.x < self.chunks_width() && coords.y < self.chunks_height() && coords.z < self.chunks_length()
    }

    #[inline(always)]
    fn debug_assert_coords_within(&self, coords: ChunkCoordinate) {
        debug_assert!(
            self.chunk_coords_within(coords),
            "{} < {} && {} < {} && {} < {} failed",
            coords.x,
            coords.y,
            coords.z,
            self.chunks_width(),
            self.chunks_height(),
            self.chunks_length()
        );
    }

    /// Returns the chunk's state
    pub fn get_chunk_state(&self, coords: ChunkCoordinate) -> ChunkState {
        if !self.chunk_coords_within(coords) {
            return ChunkState::Invalid;
        }

        if self.all_loaded {
            return ChunkState::Loaded;
        }

        let idx = self.flatten_c(coords);

        if self.loading_chunks.contains(&idx) {
            ChunkState::Loading
        } else if self.chunks.contains_key(&idx) {
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
    pub fn unload_chunk_at(&mut self, coords: ChunkCoordinate, commands: &mut Commands) -> Option<Chunk> {
        let index = self.flatten_c(coords);

        self.empty_chunks.remove(&index);
        let chunk = self.chunks.remove(&index);

        if let Some(entity) = self.chunk_entities.remove(&index) {
            commands.entity(entity).insert(NeedsDespawned);
        }

        chunk
    }

    /// Tells the structure that every chunk, empty or not, has been loaded. Do not call this
    /// manually, this should be handled automatically when the StructureLoaded event is
    /// sent. This is also not used on planets.
    ///
    /// Causes `get_chunk_state` to always return loaded unless the chunk is out of bounds.
    pub fn set_all_loaded(&mut self, all_loaded: bool) {
        self.loading_chunks.clear();
        self.all_loaded = all_loaded;
    }
}

/// Represents the state a chunk is in for loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    /// The chunk does not exist in the structure
    Invalid,
    /// The chunk does is not loaded & not being loaded
    Unloaded,
    /// The chunk is currently being loaded, but is not ready for use
    Loading,
    /// The chunk is fully loaded & ready for use
    Loaded,
}

/// This event is sent when a chunk is initially filled out
#[derive(Debug, Event)]
pub struct ChunkInitEvent {
    /// The entity of the structure this is a part of
    pub structure_entity: Entity,
    /// Chunk's coordinates in the structure
    pub coords: ChunkCoordinate,
}

// Removes chunk entities if they have no blocks
fn remove_empty_chunks(
    mut block_change_event: EventReader<BlockChangedEvent>,
    mut structure_query: Query<&mut Structure>,
    mut commands: Commands,
) {
    for bce in block_change_event.iter() {
        let Ok(mut structure) = structure_query.get_mut(bce.structure_entity) else {
            continue;
        };

        let chunk_coords = bce.block.chunk_coords();

        if structure.chunk_from_chunk_coordinates(chunk_coords).is_none() {
            if let Some(chunk_entity) = structure.chunk_entity(chunk_coords) {
                commands.entity(chunk_entity).insert(NeedsDespawned);

                let idx = structure.flatten_c(chunk_coords);
                structure.chunk_entities.remove(&idx);
            }
        }
    }
}

fn add_chunks_system(
    mut chunk_init_reader: EventReader<ChunkInitEvent>,
    mut block_reader: EventReader<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, Option<&PhysicsWorld>)>,
    mut chunk_set_event_writer: EventWriter<ChunkSetEvent>,
    mut commands: Commands,
) {
    let mut s_chunks = HashSet::new();
    let mut chunk_set_events = HashSet::new();

    for ev in block_reader.iter() {
        s_chunks.insert((ev.structure_entity, ev.block.chunk_coords()));
    }

    for ev in chunk_init_reader.iter() {
        s_chunks.insert((ev.structure_entity, ev.coords));
        chunk_set_events.insert(ChunkSetEvent {
            structure_entity: ev.structure_entity,
            coords: ev.coords,
        });
    }

    for (structure_entity, chunk_coordinate) in s_chunks {
        if let Ok((mut structure, body_world)) = structure_query.get_mut(structure_entity) {
            if let Some(chunk) = structure.chunk_from_chunk_coordinates(chunk_coordinate) {
                if !chunk.is_empty() && structure.chunk_entity(chunk_coordinate).is_none() {
                    let mut entity_cmds = commands.spawn((
                        PbrBundle {
                            transform: Transform::from_translation(structure.chunk_relative_position(chunk_coordinate)),
                            ..Default::default()
                        },
                        NoSendEntity,
                        ChunkEntity {
                            structure_entity,
                            chunk_location: chunk_coordinate,
                        },
                    ));

                    if let Some(bw) = body_world {
                        entity_cmds.insert(*bw);
                    }

                    let entity = entity_cmds.id();

                    commands.entity(structure_entity).add_child(entity);

                    structure.set_chunk_entity(chunk_coordinate, entity);

                    chunk_set_events.insert(ChunkSetEvent {
                        structure_entity,
                        coords: chunk_coordinate,
                    });
                }
            }
        }
    }

    for ev in chunk_set_events {
        chunk_set_event_writer.send(ev);
    }
}

#[derive(Debug, Clone, Copy)]
/// Represents something that went wrong when calculating the rotated coordinate for a block
pub enum RotationError {
    /// At least one of the coordinates are outside of the structure in the negative direction.
    ///
    /// Each entry represents the coordinates, even those outside.
    NegativeResult(UnboundBlockCoordinate),
    /// At least one of the coordinates are outside of the structure in the positive direction.
    ///
    /// Each entry represents the coordinates, even those outside.
    PositiveResult(BlockCoordinate),
}

impl Display for RotationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RotationError::NegativeResult(ub_coords) => f.write_str(format!("NegativeResult[{ub_coords}]").as_str()),
            RotationError::PositiveResult(coords) => f.write_str(format!("PositiveResult[{coords}]").as_str()),
        }
    }
}

/// Takes block coordinates, offsets, and the side of the planet you're on. Returns the result of applying the offsets.
/// On the +y (Top) side, the offsets affect their corresponding coordinate.
/// On other sides, the offsets affect non-corresponding coordinates and may be flipped negative.
pub fn rotate(
    block_coord: BlockCoordinate,
    delta: UnboundBlockCoordinate,
    (blocks_width, blocks_height, blocks_length): (CoordinateType, CoordinateType, CoordinateType),
    block_up: BlockFace,
) -> Result<BlockCoordinate, RotationError> {
    let ub_block_coord = UnboundBlockCoordinate::from(block_coord);

    let ub_coords = UnboundBlockCoordinate::from(match block_up {
        BlockFace::Front => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y + delta.y),
            (ub_block_coord.z + delta.z),
        ),
        BlockFace::Back => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y + delta.y),
            (ub_block_coord.z - delta.z),
        ),
        BlockFace::Top => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y + delta.y),
            (ub_block_coord.z + delta.z),
        ),
        BlockFace::Bottom => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y - delta.y),
            (ub_block_coord.z + delta.z),
        ),
        BlockFace::Right => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y + delta.y),
            (ub_block_coord.z + delta.z),
        ),
        BlockFace::Left => (
            (ub_block_coord.x - delta.x),
            (ub_block_coord.y + delta.y),
            (ub_block_coord.z + delta.z),
        ),
    });

    if let Ok(coords) = BlockCoordinate::try_from(ub_coords) {
        if coords.x >= blocks_width || coords.y >= blocks_height || coords.z >= blocks_length {
            Err(RotationError::PositiveResult(coords))
        } else {
            Ok(coords)
        }
    } else {
        Err(RotationError::NegativeResult(ub_coords))
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_game_state: T) {
    app.register_type::<Structure>()
        .register_type::<Chunk>()
        .add_event::<ChunkInitEvent>();

    systems::register(app, post_loading_state, playing_game_state);
    ship::register(app, playing_game_state);
    planet::register(app);
    events::register(app);
    loading::register(app);
    block_health::register(app);
    structure_block::register(app);

    app.add_systems(PreUpdate, (add_chunks_system, remove_empty_chunks).chain());
}
