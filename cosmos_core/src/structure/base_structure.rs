//! Internally used common logic between dynamic + full structures.

use std::{cell::RefCell, rc::Rc};

use bevy::{
    ecs::{
        component::Component,
        query::{QueryData, QueryFilter, ROQueryItem, With},
        system::{Commands, Query},
    },
    prelude::{Entity, EventWriter, GlobalTransform, Vec3},
    reflect::Reflect,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{blocks::AIR_BLOCK_ID, data::BlockData, Block, BlockRotation},
    physics::location::Location,
    registry::Registry,
};

use super::{
    block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
    block_storage::BlockStorer,
    chunk::{Chunk, CHUNK_DIMENSIONS},
    coordinates::{
        BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate, Coordinate, CoordinateType, UnboundBlockCoordinate, UnboundChunkCoordinate,
        UnboundCoordinateType,
    },
    query::MutBlockData,
    structure_block::StructureBlock,
    structure_iterator::{BlockIterator, ChunkIterator},
    BlockDataSystemParams, Structure,
};

#[derive(Reflect, Debug, Serialize, Deserialize)]
/// The most basic form of a structure. This contains shared functionality between full and dynamic structures.
///
/// Note that everything in here is subject to change
pub struct BaseStructure {
    #[serde(skip)]
    pub(super) chunk_entities: HashMap<usize, Entity>,
    #[serde(skip)]
    pub(super) chunk_entity_map: HashMap<Entity, usize>,
    #[serde(skip)]
    pub(super) self_entity: Option<Entity>,
    pub(super) chunks: HashMap<usize, Chunk>,
    dimensions: ChunkCoordinate,
}

impl BaseStructure {
    /// The most basic form of a structure. This contains shared functionality between full and dynamic structures.
    pub fn new(dimensions: ChunkCoordinate) -> Self {
        Self {
            dimensions,
            chunk_entities: Default::default(),
            chunk_entity_map: Default::default(),
            chunks: Default::default(),
            self_entity: Default::default(),
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

    #[inline]
    /// Returns true if these chunk coordinates are within the structure
    pub fn block_coords_within(&self, coords: BlockCoordinate) -> bool {
        coords.x < self.blocks_width() && coords.y < self.blocks_height() && coords.z < self.blocks_length()
    }

    #[inline]
    /// Returns true if these chunk coordinates are within the structure
    pub fn chunk_coords_within(&self, coords: ChunkCoordinate) -> bool {
        coords.x < self.chunks_width() && coords.y < self.chunks_height() && coords.z < self.chunks_length()
    }

    #[inline(always)]
    pub(super) fn debug_assert_coords_within(&self, coords: ChunkCoordinate) {
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

    pub(super) fn debug_assert_block_coords_within(&self, coords: BlockCoordinate) {
        debug_assert!(
            self.block_coords_within(coords),
            "{} < {} && {} < {} && {} < {} failed",
            coords.x,
            coords.y,
            coords.z,
            self.blocks_width(),
            self.blocks_height(),
            self.blocks_length()
        );
    }

    #[inline(always)]
    pub(super) fn flatten(&self, coords: ChunkCoordinate) -> usize {
        coords.flatten(self.dimensions.x, self.dimensions.y)
    }

    /// Returns the entity for this chunk -- an empty chunk WILL NOT have an entity.
    ///
    /// If this returns none, that means the chunk entity was not set before being used.
    #[inline(always)]
    pub fn chunk_entity(&self, coords: ChunkCoordinate) -> Option<Entity> {
        self.chunk_entities.get(&self.flatten(coords)).copied()
    }

    /// Sets the entity for the chunk at those chunk coordinates.
    ///
    /// This should be handled automatically, so you shouldn't have to call this unless
    /// you're doing some crazy stuff.
    pub fn set_chunk_entity(&mut self, coords: ChunkCoordinate, entity: Entity) {
        let index = self.flatten(coords);

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
    pub fn chunk_at(&self, coords: ChunkCoordinate) -> Option<&Chunk> {
        self.debug_assert_coords_within(coords);

        self.chunks.get(&self.flatten(coords))
    }

    /// Returns None for unloaded/empty chunks AND for chunks that are out of bounds
    ///
    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0\
    /// (-1, 0, 0) => None
    pub fn chunk_at_unbound(&self, unbound_coords: UnboundChunkCoordinate) -> Option<&Chunk> {
        let Ok(bounded) = ChunkCoordinate::try_from(unbound_coords) else {
            return None;
        };

        if self.chunk_coords_within(bounded) {
            self.chunk_at(bounded)
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
    pub fn mut_chunk_at(&mut self, coords: ChunkCoordinate) -> Option<&mut Chunk> {
        self.debug_assert_coords_within(coords);

        self.chunks.get_mut(&self.flatten(coords))
    }

    /// Returns the chunk at those block coordinates if it is non-empty AND loaded.
    ///
    /// Ex:
    /// - (0, 0, 0) => chunk @ 0, 0, 0\
    /// - (5, 0, 0) => chunk @ 0, 0, 0\
    /// - (`CHUNK_DIMENSIONS`, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at_block_coordinates(&self, coords: BlockCoordinate) -> Option<&Chunk> {
        self.chunk_at(ChunkCoordinate::for_block_coordinate(coords))
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
        self.mut_chunk_at(ChunkCoordinate::for_block_coordinate(coords))
    }

    /// Returns the number of blocks in the x, y, z direction of this structure.
    ///
    /// Valid block coordinates of this structure are from [0, [`Self::block_dimensions`])
    pub fn block_dimensions(&self) -> BlockCoordinate {
        self.dimensions.first_structure_block()
    }

    /// Returns the number of chunks in the x, y, z direction of this structure.
    pub fn chunk_dimensions(&self) -> ChunkCoordinate {
        self.dimensions
    }

    /// Returns true if these block coordinates are within the structure's bounds
    ///
    /// Note that this does not guarentee that this block location is loaded.
    pub fn is_within_blocks(&self, coords: BlockCoordinate) -> bool {
        let (w, h, l) = self.block_dimensions().into();
        coords.x < w && coords.y < h && coords.z < l
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
        let (w, h, l) = self.block_dimensions().into();
        let xx = x + (w as f32 / 2.0);
        let yy = y + (h as f32 / 2.0);
        let zz = z + (l as f32 / 2.0);

        UnboundBlockCoordinate::new(
            xx.floor() as UnboundCoordinateType,
            yy.floor() as UnboundCoordinateType,
            zz.floor() as UnboundCoordinateType,
        )
    }

    /// Gets the block's up facing face at this location.
    ///
    /// If no block was found, returns the default.
    pub fn block_rotation(&self, coords: BlockCoordinate) -> BlockRotation {
        self.chunk_at_block_coordinates(coords)
            .map(|chunk| chunk.block_rotation(ChunkBlockCoordinate::for_block_coordinate(coords)))
            .unwrap_or_default()
    }

    /// If the chunk is loaded, non-empty, returns the block at that coordinate.
    /// Otherwise, returns AIR_BLOCK_ID
    pub fn block_id_at(&self, coords: BlockCoordinate) -> u16 {
        self.debug_assert_block_coords_within(coords);

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

    /// Removes the chunk at the given coordinate -- does NOT remove the chunk entity
    pub(super) fn unload_chunk(&mut self, coords: ChunkCoordinate) {
        self.chunks.remove(&self.flatten(coords));
    }

    /// Gets the chunk's relative position to this structure's transform.
    pub fn chunk_relative_position(&self, coords: ChunkCoordinate) -> Vec3 {
        let (w, h, l) = self.dimensions.into();
        let xoff = (w as f32 - 1.0) / 2.0;
        let yoff = (h as f32 - 1.0) / 2.0;
        let zoff = (l as f32 - 1.0) / 2.0;

        let xx = CHUNK_DIMENSIONS as f32 * (coords.x as f32 - xoff);
        let yy = CHUNK_DIMENSIONS as f32 * (coords.y as f32 - yoff);
        let zz = CHUNK_DIMENSIONS as f32 * (coords.z as f32 - zoff);

        Vec3::new(xx, yy, zz)
    }

    /// Gets the block's relative position to this structure's transform.
    pub fn block_relative_position(&self, coords: BlockCoordinate) -> Vec3 {
        let xoff = self.blocks_width() as f32 / 2.0;
        let yoff = self.blocks_height() as f32 / 2.0;
        let zoff = self.blocks_length() as f32 / 2.0;

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
    /// This does not trigger any events, so make sure to handle that properly.
    pub fn set_chunk(&mut self, chunk: Chunk) {
        let i = self.flatten(chunk.chunk_coordinates());

        if chunk.is_empty() {
            self.chunks.remove(&i);
        } else {
            self.chunks.insert(i, chunk);
        }
    }

    /// Sets the chunk at this chunk location to be empty (all air).
    ///
    /// Used generally when loading stuff on client from server.
    ///
    /// This does not trigger any events, so make sure to handle those properly.
    pub fn set_to_empty_chunk(&mut self, coords: ChunkCoordinate) {
        self.chunks.remove(&self.flatten(coords));
    }

    /// # ONLY CALL THIS IF YOU THEN CALL SET_CHUNK IN THE SAME SYSTEM!
    ///
    /// This takes ownership of the chunk that was at this location. Useful for
    /// multithreading stuff over multiple chunks.
    pub fn take_chunk(&mut self, coords: ChunkCoordinate) -> Option<Chunk> {
        self.debug_assert_coords_within(coords);
        self.chunks.remove(&self.flatten(coords))
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    ///
    /// If include_empty is enabled, the value iterated over may be None OR Some(chunk).
    /// If include_empty is disabled, the value iterated over may ONLY BE Some(chunk).
    pub fn all_chunks_iter<'a>(&self, structure: &'a Structure, include_empty: bool) -> ChunkIterator<'a> {
        ChunkIterator::new(
            ChunkCoordinate::new(0, 0, 0).into(),
            ChunkCoordinate::new(self.chunks_width() - 1, self.chunks_height() - 1, self.chunks_length() - 1).into(),
            structure,
            include_empty,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn chunk_iter<'a>(
        &self,
        structure: &'a Structure,
        start: UnboundChunkCoordinate,
        end: UnboundChunkCoordinate,
        include_empty: bool,
    ) -> ChunkIterator<'a> {
        ChunkIterator::new(start, end, structure, include_empty)
    }

    /// Will fail assertion if chunk positions are out of bounds
    pub fn block_iter_for_chunk<'a>(&self, structure: &'a Structure, coords: ChunkCoordinate, include_air: bool) -> BlockIterator<'a> {
        self.debug_assert_coords_within(coords);

        BlockIterator::new(
            coords.first_structure_block().into(),
            coords.last_structure_block().into(),
            include_air,
            structure,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn all_blocks_iter<'a>(&self, structure: &'a Structure, include_air: bool) -> BlockIterator<'a> {
        BlockIterator::new(
            BlockCoordinate::new(0, 0, 0).into(),
            BlockCoordinate::new(self.blocks_width() - 1, self.blocks_height() - 1, self.blocks_length() - 1).into(),
            include_air,
            structure,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn block_iter<'a>(
        &self,
        structure: &'a Structure,
        start: UnboundBlockCoordinate,
        end: UnboundBlockCoordinate,
        include_air: bool,
    ) -> BlockIterator<'a> {
        BlockIterator::new(start, end, include_air, structure)
    }

    /// Gets the block's health at that given coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    pub fn get_block_health(&self, coords: BlockCoordinate, blocks: &Registry<Block>) -> f32 {
        self.chunk_at_block_coordinates(coords)
            .map(|c| c.get_block_health(ChunkBlockCoordinate::for_block_coordinate(coords), blocks))
            .unwrap_or(0.0)
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// - x/y/z: Block coordinates
    /// - block_hardness: The hardness for that block
    /// - amount: The amount of damage to take - cannot be negative
    ///
    /// Returns: the amount of health left over - 0.0 means the block was destroyed. None means the chunk wasn't loaded yet
    pub fn block_take_damage(
        &mut self,
        coords: BlockCoordinate,
        blocks: &Registry<Block>,
        amount: f32,
        event_writers: Option<(&mut EventWriter<BlockTakeDamageEvent>, &mut EventWriter<BlockDestroyedEvent>)>,
    ) -> Option<f32> {
        if let Some(chunk) = self.mut_chunk_at_block_coordinates(coords) {
            let health_left = chunk.block_take_damage(ChunkBlockCoordinate::for_block_coordinate(coords), amount, blocks);

            if let Some(structure_entity) = self.get_entity() {
                if let Some((take_damage_event_writer, destroyed_event_writer)) = event_writers {
                    let block = StructureBlock::new(coords);

                    take_damage_event_writer.send(BlockTakeDamageEvent {
                        structure_entity,
                        block,
                        new_health: health_left,
                    });
                    if health_left <= 0.0 {
                        destroyed_event_writer.send(BlockDestroyedEvent { structure_entity, block });
                    }
                }
            }

            Some(health_left)
        } else {
            None
        }
    }

    /// Removes the entity for this chunk - does not delete the chunk or care if the chunk even exists
    pub fn remove_chunk_entity(&mut self, coords: ChunkCoordinate) {
        self.chunk_entities.remove(&self.flatten(coords));
    }

    /// This should be used in response to a `BlockTakeDamageEvent`
    ///
    /// This will NOT delete the block if the health is 0.0
    pub(crate) fn set_block_health(&mut self, coords: BlockCoordinate, amount: f32, blocks: &Registry<Block>) {
        if let Some(chunk) = self.mut_chunk_at_block_coordinates(coords) {
            chunk.set_block_health(ChunkBlockCoordinate::for_block_coordinate(coords), amount, blocks);
        }
    }

    /// Returns `None` if the chunk is unloaded.
    ///
    /// Gets the entity that contains this block's information if there is one
    pub fn block_data(&self, coords: BlockCoordinate) -> Option<Entity> {
        if let Some(chunk) = self.chunk_at_block_coordinates(coords) {
            chunk.block_data(ChunkBlockCoordinate::for_block_coordinate(coords))
        } else {
            None
        }
    }

    /// Sets the block data entity for these coordinates.
    pub fn set_block_data_entity(&mut self, coords: BlockCoordinate, entity: Option<Entity>) {
        if let Some(chunk) = self.mut_chunk_at_block_coordinates(coords) {
            chunk.set_block_data_entity(ChunkBlockCoordinate::for_block_coordinate(coords), entity)
        }
    }

    /// Despawns any block data that is no longer used by any blocks. This should be called every frame
    /// for general cleanup and avoid systems executing on dead block-data.
    pub fn despawn_dead_block_data(&mut self, bs_commands: &mut BlockDataSystemParams) {
        for (_, chunk) in &mut self.chunks {
            chunk.despawn_dead_block_data(bs_commands);
        }
    }

    /// Returns `None` if the chunk is unloaded. Will return Some(block data entity) otherwise.
    ///
    /// Inserts data into the block here.
    pub fn insert_block_data<T: Component>(
        &mut self,
        coords: BlockCoordinate,
        data: T,
        system_params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Option<Entity> {
        let self_entity = self.get_entity()?;
        let chunk_entity = self.chunk_entity(ChunkCoordinate::for_block_coordinate(coords))?;
        let chunk = self.mut_chunk_at_block_coordinates(coords)?;

        Some(chunk.insert_block_data(
            ChunkBlockCoordinate::for_block_coordinate(coords),
            chunk_entity,
            self_entity,
            data,
            system_params,
            q_block_data,
            q_data,
        ))
    }

    /// Gets or creates the block data entity for the block here.
    ///
    /// Returns None if the chunk is not loaded here.
    pub fn get_or_create_block_data(&mut self, coords: BlockCoordinate, commands: &mut Commands) -> Option<Entity> {
        let self_entity = self.get_entity()?;
        let chunk_entity = self.chunk_entity(ChunkCoordinate::for_block_coordinate(coords))?;
        let chunk = self.mut_chunk_at_block_coordinates(coords)?;

        chunk.get_or_create_block_data(
            ChunkBlockCoordinate::for_block_coordinate(coords),
            chunk_entity,
            self_entity,
            commands,
        )
    }

    /// Gets or creates the block data entity for the block here.
    ///
    /// Returns None if the chunk is not loaded here.
    pub fn get_or_create_block_data_for_block_id(
        &mut self,
        coords: BlockCoordinate,
        block_id: u16,
        commands: &mut Commands,
    ) -> Option<Entity> {
        let self_entity = self.get_entity()?;
        let chunk_entity = self.chunk_entity(ChunkCoordinate::for_block_coordinate(coords))?;
        let chunk = self.mut_chunk_at_block_coordinates(coords)?;

        chunk.get_or_create_block_data_for_block_id(
            ChunkBlockCoordinate::for_block_coordinate(coords),
            block_id,
            chunk_entity,
            self_entity,
            commands,
        )
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
        let self_entity = self.get_entity()?;
        let chunk_entity = self.chunk_entity(ChunkCoordinate::for_block_coordinate(coords))?;
        let chunk = self.mut_chunk_at_block_coordinates(coords)?;

        Some(chunk.insert_block_data_with_entity(
            ChunkBlockCoordinate::for_block_coordinate(coords),
            chunk_entity,
            self_entity,
            create_data_closure,
            system_params,
            q_block_data,
            q_data,
        ))
    }

    /// Queries this block's data. Returns `None` if the requested query failed or if no block data exists for this block.
    pub fn query_block_data<'a, Q, F>(&'a self, coords: BlockCoordinate, query: &'a Query<Q, F>) -> Option<ROQueryItem<'a, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        let chunk = self.chunk_at_block_coordinates(coords)?;

        chunk.query_block_data(ChunkBlockCoordinate::for_block_coordinate(coords), query)
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
        let chunk = self.chunk_at_block_coordinates(coords)?;

        if let Some(e) = self.get_entity() {
            chunk.query_block_data_mut(ChunkBlockCoordinate::for_block_coordinate(coords), query, block_system_params, e)
        } else {
            None
        }
    }

    /// Removes this type of data from the block here. Returns the entity that stores this blocks data
    /// if it will still exist.
    pub fn remove_block_data<T: Component>(
        &mut self,
        coords: BlockCoordinate,
        params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Option<Entity> {
        let self_entity = self.get_entity()?;
        let chunk = self.mut_chunk_at_block_coordinates(coords)?;

        chunk.remove_block_data::<T>(
            self_entity,
            ChunkBlockCoordinate::for_block_coordinate(coords),
            params,
            q_block_data,
            q_data,
        )
    }

    /// Returns an iterator that acts as a raycast over a set of blocks in this structure
    pub fn raycast_iter(
        &self,
        start_relative_position: Vec3,
        mut direction: Vec3,
        mut max_length: f32,
        include_air: bool,
    ) -> RaycastIter<'_> {
        if direction == Vec3::ZERO {
            // If direction is zero, then the ray would never move.
            // Thus, this should only iterate over the point that is given for the start.
            return RaycastIter {
                at: start_relative_position,
                start: start_relative_position,
                base_structure: self,
                dir: Vec3::Z,
                max_length_sqrd: 0.0,
                include_air,
            };
        }

        direction = direction.normalize();

        let (min_coords, max_coords) = (
            self.block_relative_position(BlockCoordinate::new(0, 0, 0)),
            self.block_relative_position(
                BlockCoordinate::try_from(UnboundBlockCoordinate::from(self.block_dimensions()) - UnboundBlockCoordinate::new(1, 1, 1))
                    .expect("Structure cannot have dimensions of 0x0x0"),
            ),
        );

        let mut start = start_relative_position;

        if start.x < min_coords.x && direction.x > 0.0 {
            let delta_travel = min_coords.x - start.x;

            let direction_multiplier = direction.x / delta_travel;

            let start_delta = direction_multiplier * direction;

            max_length -= start_delta.length();

            start += start_delta;
        }
        if start.x > max_coords.x && direction.x < 0.0 {
            let delta_travel = min_coords.x - start.x;

            let direction_multiplier = direction.x / delta_travel;

            let start_delta = direction_multiplier * direction;

            max_length -= start_delta.length();

            start += start_delta;
        }

        if start.y < min_coords.y && direction.y > 0.0 {
            let delta_travel = min_coords.y - start.y;

            let direction_multiplier = direction.y / delta_travel;

            let start_delta = direction_multiplier * direction;

            max_length -= start_delta.length();

            start += start_delta;
        }
        if start.y > max_coords.y && direction.y < 0.0 {
            let delta_travel = min_coords.y - start.y;

            let direction_multiplier = direction.y / delta_travel;

            let start_delta = direction_multiplier * direction;

            max_length -= start_delta.length();

            start += start_delta;
        }

        if start.z < min_coords.z && direction.z > 0.0 {
            let delta_travel = min_coords.z - start.z;

            let direction_multiplier = direction.z / delta_travel;

            let start_delta = direction_multiplier * direction;

            max_length -= start_delta.length();

            start += start_delta;
        }
        if start.z > max_coords.z && direction.z < 0.0 {
            let delta_travel = min_coords.z - start.z;

            let direction_multiplier = direction.z / delta_travel;

            let start_delta = direction_multiplier * direction;

            max_length -= start_delta.length();

            start += start_delta;
        }

        let end_pos = start_relative_position + max_length * direction;
        if start.x < min_coords.x && end_pos.x < min_coords.x
            || start.y < min_coords.y && end_pos.y < min_coords.y
            || start.z < min_coords.z && end_pos.z < min_coords.z
            || start.x > max_coords.x && end_pos.x > max_coords.x
            || start.y > max_coords.y && end_pos.y > max_coords.y
            || start.z > max_coords.z && end_pos.z > max_coords.z
        {
            // This ray will never intersect this structure, so save some processing time
            // by returning an iterator that will immediately return `None`.
            return RaycastIter {
                at: start,
                start,
                base_structure: self,
                dir: direction,
                max_length_sqrd: -1.0,
                include_air,
            };
        }

        RaycastIter {
            at: start,
            start,
            base_structure: self,
            dir: direction,
            max_length_sqrd: max_length * max_length,
            include_air,
        }
    }
}

fn calculate_raycast_delta(at: Vec3, direction: Vec3) -> Vec3 {
    debug_assert_ne!(direction, Vec3::ZERO);

    let x_dec = at.x.abs() - (at.x.abs() as i32) as f32;
    let desiered_x = if direction.x < 0.0 && at.x < 0.0 {
        x_dec - 1.0
    } else if direction.x < 0.0 && at.x >= 0.0 {
        if x_dec < f32::EPSILON {
            -1.0
        } else {
            -x_dec
        }
    } else if direction.x >= 0.0 && at.x < 0.0 {
        if x_dec < f32::EPSILON {
            1.0
        } else {
            x_dec
        }
    } else {
        1.0 - x_dec
    };

    let x_amount = desiered_x / direction.x;

    let y_dec = at.y.abs() - (at.y.abs() as i32) as f32;
    let desiered_y = if direction.y < 0.0 && at.y < 0.0 {
        y_dec - 1.0
    } else if direction.y < 0.0 && at.y >= 0.0 {
        if y_dec < f32::EPSILON {
            -1.0
        } else {
            -y_dec
        }
    } else if direction.y >= 0.0 && at.y < 0.0 {
        if y_dec < f32::EPSILON {
            1.0
        } else {
            y_dec
        }
    } else {
        1.0 - y_dec
    };

    let y_amount = desiered_y / direction.y;

    let z_dec = at.z.abs() - (at.z.abs() as i32) as f32;
    let desiered_z = if direction.z < 0.0 && at.z < 0.0 {
        z_dec - 1.0
    } else if direction.z < 0.0 && at.z >= 0.0 {
        if z_dec < f32::EPSILON {
            -1.0
        } else {
            -z_dec
        }
    } else if direction.z >= 0.0 && at.z < 0.0 {
        if z_dec < f32::EPSILON {
            1.0
        } else {
            z_dec
        }
    } else {
        1.0 - z_dec
    };

    let z_amount = desiered_z / direction.z;

    let min_amount = if x_amount <= y_amount && x_amount <= z_amount {
        x_amount
    } else if y_amount <= x_amount && y_amount <= z_amount {
        y_amount
    } else {
        z_amount
    };

    min_amount * direction
}

/// Iterates over the range of blocks hit by this raycast
///
/// Create this using [`Structure::raycast_iter`]
pub struct RaycastIter<'a> {
    base_structure: &'a BaseStructure,
    start: Vec3,
    at: Vec3,
    dir: Vec3,
    max_length_sqrd: f32,
    include_air: bool,
}

impl<'a> Iterator for RaycastIter<'a> {
    type Item = BlockCoordinate;

    fn next(&mut self) -> Option<Self::Item> {
        if self.at.distance_squared(self.start) > self.max_length_sqrd {
            return None;
        }

        let mut block_id = AIR_BLOCK_ID;
        let mut n_itrs = 0;
        let mut at_coords = BlockCoordinate::new(0, 0, 0);

        while (!self.include_air && block_id == AIR_BLOCK_ID) || (n_itrs == 0) {
            let Ok(coords) = self.base_structure.relative_coords_to_local_coords_checked(
                // add just a little bit of dir to fix any rounding issues
                self.at.x + self.dir.x * 0.001,
                self.at.y + self.dir.y * 0.001,
                self.at.z + self.dir.z * 0.001,
            ) else {
                return None;
            };

            if self.at.distance_squared(self.start) > self.max_length_sqrd {
                return None;
            }

            at_coords = coords;

            let b_id = self.base_structure.block_id_at(coords);

            // Advance ray after finding next block
            self.at += calculate_raycast_delta(self.at, self.dir);

            block_id = b_id;
            n_itrs += 1;
        }

        if self.at.distance_squared(self.start) > self.max_length_sqrd {
            return None;
        }

        Some(at_coords)
    }
}

#[cfg(test)]
mod test {
    use bevy::math::Vec3;

    use super::calculate_raycast_delta;

    fn vec3_assert(a: Vec3, b: Vec3) {
        const EPSILON: f32 = 0.001;

        assert!(
            (a.x - b.x).abs() < EPSILON && (a.y - b.y).abs() < EPSILON && (a.z - b.z).abs() < EPSILON,
            "assertion `left == right` failed\n\tleft: {a:?}\n\tright: {b:?}"
        );
    }

    #[test]
    fn test_next_position_all_pos_dec() {
        let at = Vec3::new(5.5, 2.1, 2.1);

        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(6.0, 2.6, 2.6));
    }

    #[test]
    fn test_next_position_at_neg_dec() {
        let at = Vec3::new(-5.5, -2.1, -2.1);

        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(-5.4, -2.0, -2.0));
    }

    #[test]
    fn test_next_position_dir_neg_dec() {
        let at = Vec3::new(5.6, 2.1, 2.95);

        let direction = Vec3::new(-1.0, -1.0, -1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(5.5, 2.0, 2.85));
    }

    #[test]
    fn test_next_position_all_neg_dec() {
        let at = Vec3::new(-5.5, -2.1, -2.1);

        let direction = Vec3::new(-1.0, -1.0, -1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(-6.0, -2.6, -2.6));
    }

    #[test]
    fn test_next_position_all_pos_whole() {
        let at = Vec3::new(5.0, 2.1, 2.1);

        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(5.9, 3.0, 3.0));
    }

    #[test]
    fn test_next_position_all_neg_whole() {
        let at = Vec3::new(-5.0, -2.1, -2.1);

        let direction = Vec3::new(-1.0, -1.0, -1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(-5.9, -3.0, -3.0));
    }

    #[test]
    fn test_next_position_at_neg_whole() {
        let at = Vec3::new(-5.0, -2.1, -2.1);

        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(-4.9, -2.0, -2.0));
    }

    #[test]
    fn test_next_position_dir_neg_whole() {
        let at = Vec3::new(5.0, 2.1, 2.1);

        let direction = Vec3::new(-1.0, -1.0, -1.0).normalize();

        let delta_pos = calculate_raycast_delta(at, direction);

        vec3_assert(delta_pos + at, Vec3::new(4.9, 2.0, 2.0));
    }
}
