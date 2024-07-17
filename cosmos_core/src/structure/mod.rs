//! Contains all the functionality & information related to structures.
//!
//! Structures are the backbone of everything that contains blocks.

use std::cell::RefCell;
use std::fmt::Display;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use bevy::app::Update;
use bevy::ecs::query::{QueryData, QueryFilter, ROQueryItem, With};
use bevy::prelude::{App, Event, IntoSystemConfigs, Name, PreUpdate, VisibilityBundle};
use bevy::reflect::Reflect;
use bevy::transform::TransformBundle;
use bevy::utils::{HashMap, HashSet};
use bevy_rapier3d::prelude::PhysicsWorld;
use chunk::BlockInfo;
use query::MutBlockData;

pub mod asteroid;
pub mod base_structure;
pub mod block_health;
pub mod block_storage;
pub mod chunk;
pub mod coordinates;
pub mod dynamic_structure;
pub mod events;
pub mod full_structure;
pub mod loading;
pub mod lod;
pub mod lod_chunk;
pub mod planet;
pub mod query;
pub mod shared;
pub mod shields;
pub mod ship;
pub mod station;
pub mod structure_block;
pub mod structure_builder;
pub mod structure_iterator;
pub mod systems;

use crate::block::data::persistence::ChunkLoadBlockDataEvent;
use crate::block::data::BlockData;
use crate::block::{block_face::BlockFace, block_rotation::BlockRotation, Block};
use crate::ecs::NeedsDespawned;
use crate::events::block_events::{BlockChangedEvent, BlockDataChangedEvent, BlockDataSystemParams};
use crate::netty::NoSendEntity;
use crate::physics::location::Location;
use crate::registry::Registry;
use crate::structure::chunk::Chunk;
use bevy::prelude::{
    BuildChildren, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform, Query, States, Transform, Vec3,
};
use serde::{Deserialize, Serialize};

use self::base_structure::RaycastIter;
use self::block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent};
use self::block_storage::BlockStorer;
use self::chunk::netty::SerializedChunkBlockData;
use self::chunk::ChunkEntity;
use self::coordinates::{BlockCoordinate, ChunkCoordinate, UnboundBlockCoordinate, UnboundChunkCoordinate};
use self::dynamic_structure::DynamicStructure;
use self::events::ChunkSetEvent;
use self::full_structure::FullStructure;
use self::loading::StructureLoadingSet;
use self::structure_block::StructureBlock;
use self::structure_iterator::{BlockIterator, ChunkIterator};

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

#[derive(Serialize, Deserialize, Component, Reflect, Debug)]
/// A structure represents many blocks, grouped into chunks.
pub enum Structure {
    /// This structure does not have all its chunks loaded at once, such as planets
    Dynamic(DynamicStructure),
    /// This structure has all the chunks loaded at once, like ships and asteroids
    Full(FullStructure),
}

impl Structure {
    #[inline]
    /// Returns the # of chunks in the x/y/z direction as a set of ChunkCoordinates.
    pub fn chunk_dimensions(&self) -> ChunkCoordinate {
        match &self {
            Self::Dynamic(ds) => ChunkCoordinate::new(ds.chunk_dimensions(), ds.chunk_dimensions(), ds.chunk_dimensions()),
            Self::Full(full) => full.chunk_dimensions(),
        }
    }

    #[inline]
    /// Returns the # of blocks in the x/y/z direction as a set of BlockCoordinates.
    pub fn block_dimensions(&self) -> BlockCoordinate {
        match &self {
            Self::Dynamic(ds) => BlockCoordinate::new(ds.block_dimensions(), ds.block_dimensions(), ds.block_dimensions()),
            Self::Full(full) => full.block_dimensions(),
        }
    }

    /// Returns the entity for this chunk -- an empty chunk WILL NOT have an entity.
    ///
    /// If this returns none, that means the chunk entity was not set before being used.
    /// Maybe the chunk is empty or unloaded?
    #[inline]
    pub fn chunk_entity(&self, coords: ChunkCoordinate) -> Option<Entity> {
        match self {
            Self::Dynamic(ds) => ds.chunk_entity(coords),
            Self::Full(full) => full.chunk_entity(coords),
        }
    }

    /// Sets the entity for the chunk at those chunk coordinates.
    ///
    /// This should be handled automatically, so you shouldn't have to call this unless
    /// you're doing some crazy stuff.
    pub fn set_chunk_entity(&mut self, coords: ChunkCoordinate, entity: Entity) {
        match self {
            Self::Dynamic(ds) => ds.set_chunk_entity(coords, entity),
            Self::Full(fs) => fs.set_chunk_entity(coords, entity),
        }
    }

    /// Gets the chunk from its entity, or return None if there is no loaded chunk for that entity.
    ///
    /// Remember that empty chunks will NOT have an entity.
    pub fn chunk_from_entity(&self, entity: &Entity) -> Option<&Chunk> {
        match self {
            Self::Dynamic(ds) => ds.chunk_from_entity(entity),
            Self::Full(fs) => fs.chunk_from_entity(entity),
        }
    }

    /// Sets this structure's entity - used in the base builder.
    pub(crate) fn set_entity(&mut self, entity: Entity) {
        match self {
            Self::Dynamic(ds) => ds.set_entity(entity),
            Self::Full(fs) => fs.set_entity(entity),
        }
    }

    /// Gets the structure's entity
    ///
    /// May be None if this hasn't been built yet.
    pub fn get_entity(&self) -> Option<Entity> {
        match self {
            Self::Dynamic(ds) => ds.get_entity(),
            Self::Full(fs) => fs.get_entity(),
        }
    }

    /// Returns None for unloaded/empty chunks or chunks that are out of bounds
    ///  
    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at(&self, coords: ChunkCoordinate) -> Option<&Chunk> {
        match self {
            Self::Dynamic(ds) => ds.chunk_at(coords),
            Self::Full(fs) => fs.chunk_at(coords),
        }
    }

    /// Returns None for unloaded/empty chunks AND for chunks that are out of bounds
    ///
    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0\
    /// (-1, 0, 0) => None
    pub fn chunk_at_unbound(&self, unbound_coords: UnboundChunkCoordinate) -> Option<&Chunk> {
        match self {
            Self::Full(fs) => fs.chunk_at_unbound(unbound_coords),
            Self::Dynamic(ds) => ds.chunk_at_unbound(unbound_coords),
        }
    }

    /// Gets the mutable chunk for these chunk coordinates. If the chunk is unloaded, empty, or out of bounds, this will return None.
    ///
    /// ## Be careful with this!!
    ///
    /// Modifying a chunk will not update the structure or chunks surrounding it and it won't send any events.
    /// Unless you know what you're doing, you should use a mutable structure instead
    /// of a mutable chunk to make changes!
    pub fn mut_chunk_at(&mut self, coords: ChunkCoordinate) -> Option<&mut Chunk> {
        match self {
            Self::Full(fs) => fs.mut_chunk_at(coords),
            Self::Dynamic(ds) => ds.mut_chunk_at(coords),
        }
    }

    /// Returns the chunk at those block coordinates if it is non-empty AND loaded.
    ///
    /// Ex:
    /// - (0, 0, 0) => chunk @ 0, 0, 0\
    /// - (5, 0, 0) => chunk @ 0, 0, 0\
    /// - (`CHUNK_DIMENSIONS`, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at_block_coordinates(&self, coords: BlockCoordinate) -> Option<&Chunk> {
        match self {
            Self::Full(fs) => fs.chunk_at_block_coordinates(coords),
            Self::Dynamic(ds) => ds.chunk_at_block_coordinates(coords),
        }
    }

    /// Returns true if these block coordinates are within the structure's bounds
    ///
    /// Note that this does not guarentee that this block location is loaded.
    pub fn is_within_blocks(&self, coords: BlockCoordinate) -> bool {
        match self {
            Self::Full(fs) => fs.is_within_blocks(coords),
            Self::Dynamic(ds) => ds.is_within_blocks(coords),
        }
    }

    /// Returns true if the structure has a loaded block here that isn't air.
    pub fn has_block_at(&self, coords: BlockCoordinate) -> bool {
        match self {
            Self::Full(fs) => fs.has_block_at(coords),
            Self::Dynamic(ds) => ds.has_block_at(coords),
        }
    }

    /// # Arguments
    /// Coordinates relative to the structure's 0, 0, 0 position in the world mapped to block coordinates
    /// # Returns
    /// - Ok (x, y, z) of the block coordinates if the point is within the structure
    /// - Err(false) if one of the x/y/z coordinates are outside the structure in the negative direction
    /// - Err (true) if one of the x/y/z coordinates are outside the structure in the positive direction
    pub fn relative_coords_to_local_coords_checked(&self, x: f32, y: f32, z: f32) -> Result<BlockCoordinate, bool> {
        match self {
            Self::Full(fs) => fs.relative_coords_to_local_coords_checked(x, y, z),
            Self::Dynamic(ds) => ds.relative_coords_to_local_coords_checked(x, y, z),
        }
    }

    /// # Arguments
    /// Coordinates relative to the structure's 0, 0, 0 position in the world mapped to block coordinates.
    ///
    /// These coordinates may not be within the structure (too high or negative).
    /// # Returns
    /// - (x, y, z) of the block coordinates, even if they are outside the structure
    pub fn relative_coords_to_local_coords(&self, x: f32, y: f32, z: f32) -> UnboundBlockCoordinate {
        match self {
            Self::Full(fs) => fs.relative_coords_to_local_coords(x, y, z),
            Self::Dynamic(ds) => ds.relative_coords_to_local_coords(x, y, z),
        }
    }

    /// Gets the block's up facing face at this location.
    ///
    /// If no block was found, returns BlockFace::Top.
    pub fn block_rotation(&self, coords: BlockCoordinate) -> BlockRotation {
        match self {
            Self::Full(fs) => fs.block_rotation(coords),
            Self::Dynamic(ds) => ds.block_rotation(coords),
        }
    }

    /// If the chunk is loaded, non-empty, returns the block at that coordinate.
    /// Otherwise, returns AIR_BLOCK_ID
    pub fn block_id_at(&self, coords: BlockCoordinate) -> u16 {
        match self {
            Self::Full(fs) => fs.block_id_at(coords),
            Self::Dynamic(ds) => ds.block_id_at(coords),
        }
    }

    /// Gets the block at these block coordinates
    pub fn block_at<'a>(&'a self, coords: BlockCoordinate, blocks: &'a Registry<Block>) -> &'a Block {
        match self {
            Self::Full(fs) => fs.block_at(coords, blocks),
            Self::Dynamic(ds) => ds.block_at(coords, blocks),
        }
    }

    /// Gets the hashmap for the loaded, non-empty chunks.
    ///
    /// This is going to be replaced with an iterator in the future
    pub fn chunks(&self) -> &HashMap<usize, Chunk> {
        match self {
            Self::Full(fs) => fs.chunks(),
            Self::Dynamic(ds) => ds.chunks(),
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
        match self {
            Self::Full(fs) => fs.remove_block_at(coords, blocks, event_writer),
            Self::Dynamic(ds) => ds.remove_block_at(coords, blocks, event_writer),
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
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        match self {
            Self::Full(fs) => fs.set_block_at(coords, block, block_rotation, blocks, event_writer),
            Self::Dynamic(ds) => ds.set_block_at(coords, block, block_rotation, blocks, event_writer),
        }
    }

    /// Gets the chunk's relative position to this structure's transform.
    pub fn chunk_relative_position(&self, coords: ChunkCoordinate) -> Vec3 {
        match self {
            Self::Full(fs) => fs.chunk_relative_position(coords),
            Self::Dynamic(ds) => ds.chunk_relative_position(coords),
        }
    }

    /// Gets the block's relative position to this structure's transform.
    pub fn block_relative_position(&self, coords: BlockCoordinate) -> Vec3 {
        match self {
            Self::Full(fs) => fs.block_relative_position(coords),
            Self::Dynamic(ds) => ds.block_relative_position(coords),
        }
    }

    /// Gets a blocks's location in the world
    pub fn block_world_location(&self, coords: BlockCoordinate, body_position: &GlobalTransform, this_location: &Location) -> Location {
        match self {
            Self::Full(fs) => fs.block_world_location(coords, body_position, this_location),
            Self::Dynamic(ds) => ds.block_world_location(coords, body_position, this_location),
        }
    }

    /// Sets the chunk, overwriting what may have been there before.
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle that properly.
    pub fn set_chunk(&mut self, chunk: Chunk) {
        match self {
            Self::Full(fs) => fs.set_chunk(chunk),
            Self::Dynamic(ds) => ds.set_chunk(chunk),
        }
    }

    /// Sets the chunk at this chunk location to be empty (all air).
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle those properly.
    pub fn set_to_empty_chunk(&mut self, coords: ChunkCoordinate) {
        match self {
            Self::Full(fs) => fs.set_to_empty_chunk(coords),
            Self::Dynamic(ds) => ds.set_to_empty_chunk(coords),
        }
    }

    /// # ONLY CALL THIS IF YOU THEN CALL SET_CHUNK IN THE SAME SYSTEM!
    ///
    /// This takes ownership of the chunk that was at this location. Useful for
    /// multithreading stuff over multiple chunks.
    pub fn take_chunk(&mut self, coords: ChunkCoordinate) -> Option<Chunk> {
        match self {
            Self::Full(fs) => fs.take_chunk(coords),
            Self::Dynamic(ds) => ds.take_chunk(coords),
        }
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    ///
    /// If include_empty is enabled, the value iterated over may be None OR Some(chunk).
    /// If include_empty is disabled, the value iterated over may ONLY BE Some(chunk).
    pub fn all_chunks_iter(&self, include_empty: bool) -> ChunkIterator {
        match self {
            Self::Full(fs) => fs.all_chunks_iter(self, include_empty),
            Self::Dynamic(ds) => ds.all_chunks_iter(self, include_empty),
        }
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn chunk_iter(&self, start: UnboundChunkCoordinate, end: UnboundChunkCoordinate, include_empty: bool) -> ChunkIterator {
        match self {
            Self::Full(fs) => fs.chunk_iter(self, start, end, include_empty),
            Self::Dynamic(ds) => ds.chunk_iter(self, start, end, include_empty),
        }
    }

    /// Will fail assertion if chunk positions are out of bounds
    pub fn block_iter_for_chunk(&self, coords: ChunkCoordinate, include_air: bool) -> BlockIterator {
        match self {
            Self::Full(fs) => fs.block_iter_for_chunk(self, coords, include_air),
            Self::Dynamic(ds) => ds.block_iter_for_chunk(self, coords, include_air),
        }
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn all_blocks_iter(&self, include_air: bool) -> BlockIterator {
        match self {
            Self::Full(fs) => fs.all_blocks_iter(self, include_air),
            Self::Dynamic(ds) => ds.all_blocks_iter(self, include_air),
        }
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn block_iter(&self, start: UnboundBlockCoordinate, end: UnboundBlockCoordinate, include_air: bool) -> BlockIterator {
        match self {
            Self::Full(fs) => fs.block_iter(self, start, end, include_air),
            Self::Dynamic(ds) => ds.block_iter(self, start, end, include_air),
        }
    }

    /// Gets the block's health at that given coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    pub fn get_block_health(&self, coords: BlockCoordinate, blocks: &Registry<Block>) -> f32 {
        match self {
            Self::Full(fs) => fs.get_block_health(coords, blocks),
            Self::Dynamic(ds) => ds.get_block_health(coords, blocks),
        }
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
        blocks: &Registry<Block>,
        amount: f32,
        event_writers: Option<(&mut EventWriter<BlockTakeDamageEvent>, &mut EventWriter<BlockDestroyedEvent>)>,
    ) -> Option<f32> {
        match self {
            Self::Full(fs) => fs.block_take_damage(coords, blocks, amount, event_writers),
            Self::Dynamic(ds) => ds.block_take_damage(coords, blocks, amount, event_writers),
        }
    }

    /// This should be used in response to a `BlockTakeDamageEvent`
    ///
    /// # This will NOT delete the block if the health is 0.0
    pub fn set_block_health(&mut self, coords: BlockCoordinate, amount: f32, blocks: &Registry<Block>) {
        debug_assert!(amount != 0.0, "Block health cannot be 0.0!");

        match self {
            Self::Full(fs) => fs.set_block_health(coords, amount, blocks),
            Self::Dynamic(ds) => ds.set_block_health(coords, amount, blocks),
        }
    }

    /// Gets the chunk's state
    pub fn get_chunk_state(&self, coords: ChunkCoordinate) -> ChunkState {
        match self {
            Self::Full(fs) => fs.get_chunk_state(coords),
            Self::Dynamic(ds) => ds.get_chunk_state(coords),
        }
    }

    #[inline]
    /// Returns true if these chunk coordinates are within the structure
    pub fn chunk_coords_within(&self, coords: ChunkCoordinate) -> bool {
        match self {
            Self::Dynamic(ds) => ds.chunk_coords_within(coords),
            Self::Full(fs) => fs.chunk_coords_within(coords),
        }
    }

    /// Removes ths chunk entity from the structure
    pub fn remove_chunk_entity(&mut self, coords: ChunkCoordinate) {
        match self {
            Self::Full(fs) => fs.remove_chunk_entity(coords),
            Self::Dynamic(ds) => ds.remove_chunk_entity(coords),
        }
    }

    /// Returns true if this structure has a loaded empty chunk at these coordinates.
    ///
    /// Will return false for unloaded chunks.
    pub fn has_empty_chunk_at(&self, coords: ChunkCoordinate) -> bool {
        match self {
            Self::Full(fs) => fs.has_empty_chunk_at(coords),
            Self::Dynamic(ds) => ds.has_empty_chunk_at(coords),
        }
    }

    /// Returns `None` if the chunk is unloaded.
    ///
    /// Inserts data into the block here.
    ///
    /// If you need to know the block data entity to construct the
    /// block data, use [`Self::insert_block_data_with_entity`] instead.
    pub fn insert_block_data<T: Component>(
        &mut self,
        coords: BlockCoordinate,
        data: T,
        system_params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_has_data: &Query<(), With<T>>,
    ) -> Option<Entity> {
        match self {
            Self::Full(fs) => fs.insert_block_data(coords, data, system_params, q_block_data, q_has_data),
            Self::Dynamic(ds) => ds.insert_block_data(coords, data, system_params, q_block_data, q_has_data),
        }
    }

    /// Gets or creates the block data entity for the block here.
    ///
    /// Returns None if the chunk is not loaded here.
    pub fn get_or_create_block_data(&mut self, coords: BlockCoordinate, commands: &mut Commands) -> Option<Entity> {
        match self {
            Self::Full(fs) => fs.get_or_create_block_data(coords, commands),
            Self::Dynamic(ds) => ds.get_or_create_block_data(coords, commands),
        }
    }

    /// Gets or creates the block data entity for the block here.
    ///
    /// Returns None if the chunk is not loaded here.
    ///
    /// Used exclusively for syncing from server -> client.
    pub fn get_or_create_block_data_for_block_id(
        &mut self,
        coords: BlockCoordinate,
        block_id: u16,
        commands: &mut Commands,
    ) -> Option<Entity> {
        match self {
            Self::Full(fs) => fs.get_or_create_block_data_for_block_id(coords, block_id, commands),
            Self::Dynamic(ds) => ds.get_or_create_block_data_for_block_id(coords, block_id, commands),
        }
    }

    /// Returns `None` if the chunk is unloaded.
    ///
    /// Inserts data into the block here. This differs from the
    /// normal [`Self::insert_block_data`] in that it will call the closure
    /// with the block data entity to create the data to insert.
    ///
    /// This is useful for things such as Inventories, which require the entity
    /// that is storing them in their constructor method.
    pub fn insert_block_data_with_entity<T: Component, F>(
        &mut self,
        coords: BlockCoordinate,
        create_data_closure: F,
        system_params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Option<Entity>
    where
        F: FnOnce(Entity) -> T,
    {
        match self {
            Self::Full(fs) => fs.insert_block_data_with_entity(coords, create_data_closure, system_params, q_block_data, q_data),
            Self::Dynamic(ds) => ds.insert_block_data_with_entity(coords, create_data_closure, system_params, q_block_data, q_data),
        }
    }

    /// Queries this block's data. Returns `None` if the requested query failed or if no block data exists for this block.
    pub fn query_block_data<'a, Q, F>(&'a self, coords: BlockCoordinate, query: &'a Query<Q, F>) -> Option<ROQueryItem<'a, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        match self {
            Self::Full(fs) => fs.query_block_data(coords, query),
            Self::Dynamic(ds) => ds.query_block_data(coords, query),
        }
    }

    /// Queries this block's data mutibly. Returns `None` if the requested query failed or if no block data exists for this block.
    pub fn query_block_data_mut<'q, 'w, 's, Q, F>(
        &'q self,
        coords: BlockCoordinate,
        query: &'q mut Query<Q, F>,
        block_system_params: Rc<RefCell<BlockDataSystemParams<'w, 's>>>,
    ) -> Option<MutBlockData<'q, 'w, 's, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        match self {
            Self::Full(fs) => fs.query_block_data_mut(coords, query, block_system_params),
            Self::Dynamic(ds) => ds.query_block_data_mut(coords, query, block_system_params),
        }
    }

    /// Returns `None` if the chunk is unloaded.
    ///
    /// Gets the entity that contains this block's information if there is one
    pub fn block_data(&self, coords: BlockCoordinate) -> Option<Entity> {
        match self {
            Self::Full(fs) => fs.block_data(coords),
            Self::Dynamic(ds) => ds.block_data(coords),
        }
    }

    /// Sets the block data entity for these coordinates.
    pub fn set_block_data_entity(&mut self, coords: BlockCoordinate, entity: Option<Entity>) {
        match self {
            Self::Full(fs) => fs.set_block_data_entity(coords, entity),
            Self::Dynamic(ds) => ds.set_block_data_entity(coords, entity),
        }
    }

    /// Despawns any block data that is no longer used by any blocks. This should be called every frame
    /// for general cleanup and avoid systems executing on dead block-data.
    pub fn despawn_dead_block_data(&mut self, bs_commands: &mut BlockDataSystemParams) {
        match self {
            Self::Full(fs) => fs.despawn_dead_block_data(bs_commands),
            Self::Dynamic(ds) => ds.despawn_dead_block_data(bs_commands),
        }
    }

    /// Removes any block data associated with this block
    ///
    /// Will return the data entity that was previously here, if any
    pub fn remove_block_data<T: Component>(
        &mut self,
        coords: BlockCoordinate,
        params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Option<Entity> {
        match self {
            Self::Full(fs) => fs.remove_block_data(coords, params, q_block_data, q_data),
            Self::Dynamic(ds) => ds.remove_block_data(coords, params, q_block_data, q_data),
        }
    }

    #[must_use]
    /// Returns an iterator that acts as a raycast over a set of blocks in this structure
    pub fn raycast_iter(&self, start_relative_position: Vec3, direction: Vec3, max_length: f32, include_air: bool) -> RaycastIter<'_> {
        match self {
            Self::Full(fs) => fs.raycast_iter(start_relative_position, direction, max_length, include_air),
            Self::Dynamic(ds) => ds.raycast_iter(start_relative_position, direction, max_length, include_air),
        }
    }

    /// Returns the small block information storage (for example, rotation) for this block within the chunk.
    pub fn block_info_at(&self, coords: BlockCoordinate) -> BlockInfo {
        match self {
            Self::Full(fs) => fs.block_info_at(coords),
            Self::Dynamic(ds) => ds.block_info_at(coords),
        }
    }

    /// Sets the small block information storage (for example, rotation) for this block within the chunk.
    pub fn set_block_info_at(
        &mut self,
        coords: BlockCoordinate,
        block_info: BlockInfo,
        evw_block_data_changed: &mut EventWriter<BlockDataChangedEvent>,
    ) {
        match self {
            Self::Full(fs) => fs.set_block_info_at(coords, block_info, evw_block_data_changed),
            Self::Dynamic(ds) => ds.set_block_info_at(coords, block_info, evw_block_data_changed),
        }
    }

    /// Returns the 6 block IDs adjacent to the given coordinates.
    /// Any error causes that entry to be AIR_BLOCK_ID.
    pub fn block_ids_surrounding(&self, coords: BlockCoordinate) -> [u16; 6] {
        match self {
            Self::Full(fs) => fs.block_ids_surrounding(coords),
            Self::Dynamic(ds) => ds.block_ids_surrounding(coords),
        }
    }

    /// Returns the 6 blocks adjacent to the given coordinates.
    /// Any error causes that entry to be air.
    pub fn blocks_surrounding<'a>(&self, coords: BlockCoordinate, blocks: &'a Registry<Block>) -> [&'a Block; 6] {
        match self {
            Self::Full(fs) => fs.blocks_surrounding(coords, blocks),
            Self::Dynamic(ds) => ds.blocks_surrounding(coords, blocks),
        }
    }
}

/// This event is sent when a chunk is initially filled out
#[derive(Debug, Event)]
pub struct ChunkInitEvent {
    /// The entity of the structure this is a part of
    pub structure_entity: Entity,
    /// Chunk's coordinates in the structure
    pub coords: ChunkCoordinate,
    /// If the chunk has block data that needs deserialized, this will be populated
    ///
    /// Arc<Mutex<>> so we can efficiently take the data without cloning it
    pub serialized_block_data: Option<Arc<Mutex<SerializedChunkBlockData>>>,
}

// Removes chunk entities if they have no blocks
fn remove_empty_chunks(
    mut block_change_event: EventReader<BlockChangedEvent>,
    mut structure_query: Query<&mut Structure>,
    mut commands: Commands,
) {
    for bce in block_change_event.read() {
        let Ok(mut structure) = structure_query.get_mut(bce.structure_entity) else {
            continue;
        };

        let chunk_coords = bce.block.chunk_coords();

        if structure.chunk_at(chunk_coords).is_none() {
            if let Some(chunk_entity) = structure.chunk_entity(chunk_coords) {
                commands.entity(chunk_entity).insert(NeedsDespawned);

                structure.remove_chunk_entity(chunk_coords);
            }
        }
    }
}

fn spawn_chunk_entity(
    commands: &mut Commands,
    structure: &mut Structure,
    chunk_coordinate: ChunkCoordinate,
    structure_entity: Entity,
    body_world: Option<&PhysicsWorld>,
    chunk_set_events: &mut HashSet<ChunkSetEvent>,
) {
    let mut entity_cmds = commands.spawn((
        VisibilityBundle::default(),
        TransformBundle::from_transform(Transform::from_translation(structure.chunk_relative_position(chunk_coordinate))),
        Name::new("Chunk Entity"),
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

fn add_chunks_system(
    mut chunk_init_reader: EventReader<ChunkInitEvent>,
    mut block_reader: EventReader<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, Option<&PhysicsWorld>)>,
    mut chunk_set_event_writer: EventWriter<ChunkSetEvent>,
    mut commands: Commands,
    mut ev_writer: EventWriter<ChunkLoadBlockDataEvent>,
) {
    let mut s_chunks = HashSet::new();
    let mut chunk_set_events = HashSet::new();

    for ev in block_reader.read() {
        s_chunks.insert((ev.structure_entity, ev.block.chunk_coords()));
    }

    for ev in chunk_init_reader.read() {
        s_chunks.insert((ev.structure_entity, ev.coords));
        chunk_set_events.insert(ChunkSetEvent {
            structure_entity: ev.structure_entity,
            coords: ev.coords,
        });

        if let Some(serialized_data) = &ev.serialized_block_data {
            let mut data: std::sync::MutexGuard<'_, SerializedChunkBlockData> = serialized_data.lock().unwrap();
            let data = data.deref_mut();

            let data = std::mem::take(data);

            ev_writer.send(ChunkLoadBlockDataEvent {
                data,
                chunk: ev.coords,
                structure_entity: ev.structure_entity,
            });
        }
    }

    for (structure_entity, chunk_coordinate) in s_chunks {
        let Ok((mut structure, body_world)) = structure_query.get_mut(structure_entity) else {
            continue;
        };

        let Some(chunk) = structure.chunk_at(chunk_coordinate) else {
            continue;
        };

        if !chunk.is_empty() && structure.chunk_entity(chunk_coordinate).is_none() {
            spawn_chunk_entity(
                &mut commands,
                &mut structure,
                chunk_coordinate,
                structure_entity,
                body_world,
                &mut chunk_set_events,
            );
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
    dimensions: BlockCoordinate,
    block_up: BlockFace,
) -> Result<BlockCoordinate, RotationError> {
    let ub_block_coord = UnboundBlockCoordinate::from(block_coord);

    let ub_coords = UnboundBlockCoordinate::from(match block_up {
        BlockFace::Back => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y + delta.z),
            (ub_block_coord.z + delta.y),
        ),
        BlockFace::Front => (
            (ub_block_coord.x + delta.x),
            (ub_block_coord.y + delta.z),
            (ub_block_coord.z - delta.y),
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
            (ub_block_coord.x + delta.y),
            (ub_block_coord.y + delta.x),
            (ub_block_coord.z + delta.z),
        ),
        BlockFace::Left => (
            (ub_block_coord.x - delta.y),
            (ub_block_coord.y + delta.x),
            (ub_block_coord.z + delta.z),
        ),
    });

    if let Ok(coords) = BlockCoordinate::try_from(ub_coords) {
        if coords.x >= dimensions.x || coords.y >= dimensions.y || coords.z >= dimensions.z {
            Err(RotationError::PositiveResult(coords))
        } else {
            Ok(coords)
        }
    } else {
        Err(RotationError::NegativeResult(ub_coords))
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.register_type::<Structure>()
        .register_type::<Chunk>()
        .add_event::<ChunkInitEvent>();

    ship::register(app, playing_state);
    station::register(app, playing_state);
    chunk::register(app);
    planet::register(app);
    events::register(app);
    shared::register(app);
    loading::register(app);
    systems::register(app);
    shields::register(app);
    block_health::register(app);
    structure_block::register(app);

    app.add_systems(Update, add_chunks_system.in_set(StructureLoadingSet::CreateChunkEntities))
        .add_systems(PreUpdate, remove_empty_chunks);
}
