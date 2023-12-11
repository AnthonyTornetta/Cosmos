//! Represents a fixed region of blocks.
//!
//! These blocks can be updated.

use bevy::prelude::{App, Component, Entity, Event, Vec3};
use bevy::reflect::Reflect;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::block::{Block, BlockFace};
use crate::registry::Registry;

use super::block_health::BlockHealth;
use super::block_storage::{BlockStorage, BlockStorer};
use super::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType, UnboundCoordinateType};

pub mod netty;

/// The number of blocks a chunk can have in the x/y/z directions.
///
/// A chunk contains `CHUNK_DIMENSIONS`^3 blocks total.
pub const CHUNK_DIMENSIONS: CoordinateType = 32;

/// Short for `CHUNK_DIMENSIONS as usize`
pub const CHUNK_DIMENSIONS_USIZE: usize = CHUNK_DIMENSIONS as usize;

/// Short for `CHUNK_DIMENSIONS as f32`
pub const CHUNK_DIMENSIONSF: f32 = CHUNK_DIMENSIONS as f32;

/// Short for `CHUNK_DIMENSIONS as UnboundCoordinateType`
pub const CHUNK_DIMENSIONS_UB: UnboundCoordinateType = CHUNK_DIMENSIONS as UnboundCoordinateType;

#[derive(Debug, Reflect, Serialize, Deserialize, Clone)]
/// Stores a bunch of blocks, information about those blocks, and where they are in the structure.
pub struct Chunk {
    structure_position: ChunkCoordinate,
    block_health: BlockHealth,

    block_storage: BlockStorage,

    /// Each entity this points to should ideally be a child of this chunk used to store data about a specific block
    #[serde(skip)]
    block_data: HashMap<ChunkBlockCoordinate, Entity>,
}

impl BlockStorer for Chunk {
    #[inline(always)]
    fn block_at(&self, coords: ChunkBlockCoordinate) -> u16 {
        self.block_storage.block_at(coords)
    }

    #[inline(always)]
    fn block_info_iterator(&self) -> std::slice::Iter<BlockInfo> {
        self.block_storage.block_info_iterator()
    }

    #[inline(always)]
    fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockFace {
        self.block_storage.block_rotation(coords)
    }

    #[inline(always)]
    fn blocks(&self) -> std::slice::Iter<u16> {
        self.block_storage.blocks()
    }

    #[inline(always)]
    fn debug_assert_is_within_blocks(&self, coords: ChunkBlockCoordinate) {
        self.block_storage.debug_assert_is_within_blocks(coords)
    }

    #[inline(always)]
    fn has_block_at(&self, coords: ChunkBlockCoordinate) -> bool {
        self.block_storage.has_block_at(coords)
    }

    #[inline(always)]
    fn has_full_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        self.block_storage.has_full_block_at(coords, blocks)
    }

    #[inline(always)]
    fn has_see_through_block_at(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> bool {
        self.block_storage.has_see_through_block_at(coords, blocks)
    }

    #[inline(always)]
    fn is_empty(&self) -> bool {
        self.block_storage.is_empty()
    }

    #[inline(always)]
    fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_up: BlockFace) {
        self.block_storage.set_block_at(coords, b, block_up)
    }

    #[inline(always)]
    fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_up: BlockFace) {
        self.block_storage.set_block_at_from_id(coords, id, block_up)
    }

    #[inline(always)]
    fn is_within_blocks(&self, coords: ChunkBlockCoordinate) -> bool {
        self.block_storage.is_within_blocks(coords)
    }
}

impl Chunk {
    /// Creates a chunk containing all air blocks.
    ///
    /// * `x` The x chunk location in the structure
    /// * `y` The y chunk location in the structure
    /// * `z` The z chunk location in the structure
    pub fn new(structure_position: ChunkCoordinate) -> Self {
        Self {
            structure_position,
            block_storage: BlockStorage::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS),
            block_health: BlockHealth::default(),
            block_data: Default::default(),
        }
    }

    #[inline]
    /// The position of this chunk in the structure.
    pub fn chunk_coordinates(&self) -> ChunkCoordinate {
        self.structure_position
    }

    #[inline]
    /// The position in the structure x.
    pub fn structure_x(&self) -> CoordinateType {
        self.structure_position.x
    }

    #[inline]
    /// The position in the structure y.
    pub fn structure_y(&self) -> CoordinateType {
        self.structure_position.y
    }

    #[inline]
    /// The position in the structure z.
    pub fn structure_z(&self) -> CoordinateType {
        self.structure_position.z
    }

    #[inline(always)]
    /// Debug asserts that coordinates are within a chunk
    ///
    /// Will panic in debug mode if they are not
    pub fn debug_assert_is_within_blocks(coords: ChunkBlockCoordinate) {
        debug_assert!(
            coords.x < CHUNK_DIMENSIONS && coords.y < CHUNK_DIMENSIONS && coords.z < CHUNK_DIMENSIONS,
            "{} < {CHUNK_DIMENSIONS} && {} < {CHUNK_DIMENSIONS} && {} < {CHUNK_DIMENSIONS} failed",
            coords.x,
            coords.y,
            coords.z,
        );
    }

    /// Calculates the block coordinates used in something like `Self::block_at` from their f32 coordinates relative to the chunk's center.
    pub fn relative_coords_to_block_coords(&self, relative: &Vec3) -> (usize, usize, usize) {
        (
            (relative.x + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
            (relative.y + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
            (relative.z + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
        )
    }

    /// Gets the block's health at that given coordinate
    /// * `x/y/z`: block coordinate
    /// * `block_hardness`: The hardness for the block at those coordinates
    pub fn get_block_health(&self, coords: ChunkBlockCoordinate, blocks: &Registry<Block>) -> f32 {
        self.block_health
            .get_health(coords, blocks.from_numeric_id(self.block_at(coords)).hardness())
    }

    /// Causes a block at the given coordinates to take damage
    ///
    /// * `x/y/z` Block coordinates
    /// * `block_hardness` The hardness for that block
    /// * `amount` The amount of damage to take - cannot be negative
    ///
    /// **Returns:** The leftover health - 0.0 means the block was destroyed
    pub fn block_take_damage(&mut self, coords: ChunkBlockCoordinate, amount: f32, blocks: &Registry<Block>) -> f32 {
        self.block_health
            .take_damage(coords, blocks.from_numeric_id(self.block_at(coords)).hardness(), amount)
    }

    /// This should be used in response to a `BlockTakeDamageEvent`
    ///
    /// This will NOT delete the block if the health is 0.0
    pub(crate) fn set_block_health(&mut self, coords: ChunkBlockCoordinate, amount: f32, blocks: &Registry<Block>) {
        self.block_health
            .set_health(coords, blocks.from_numeric_id(self.block_at(coords)).hardness(), amount);
    }

    /// Gets the entity that contains this block's information if there is one
    pub fn block_data(&self, coords: ChunkBlockCoordinate) -> Option<Entity> {
        self.block_data.get(&coords).copied()
    }

    /// Sets the block at these coordinate's data.
    ///
    /// This does NOT despawn previous data that was here.
    ///
    /// Will return the entity that was previously here, if any
    pub fn set_block_data(&mut self, coords: ChunkBlockCoordinate, data_entity: Entity) -> Option<Entity> {
        self.block_data.insert(coords, data_entity)
    }

    /// Removes any block data associated with this block
    ///
    /// Will return the data entity that was previously here, if any
    pub fn remove_block_data(&mut self, coords: ChunkBlockCoordinate) -> Option<Entity> {
        self.block_data.remove(&coords)
    }

    /// Returns all the block data entities this chunk has.
    ///
    /// Mostly just used for saving
    pub fn all_block_data_entities(&self) -> &HashMap<ChunkBlockCoordinate, Entity> {
        &self.block_data
    }
}

#[derive(Debug, Default, Reflect, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
/// This represents the information for a block. The first 3 rightmost bits are reserved for rotation data.
///
/// All other bits can be used for anything else
pub struct BlockInfo(u8);

impl BlockInfo {
    #[inline]
    /// Gets the rotation data
    ///
    /// This will return which BlockFace represents the UP direction (no rotation is BlockFace::Top)
    pub fn get_rotation(&self) -> BlockFace {
        BlockFace::from_index((self.0 & 0b111) as usize)
    }

    /// Sets the rotation data
    ///
    /// This should be the BlockFace that represents the UP direction (no rotation is BlockFace::Top)
    pub fn set_rotation(&mut self, rotation: BlockFace) {
        self.0 = self.0 & !0b111 | rotation.index() as u8;
    }
}

/// Represents a child of a structure that represents a chunk
#[derive(Debug, Reflect, Component)]
pub struct ChunkEntity {
    /// The entity of the structure this is a part of
    pub structure_entity: Entity,
    /// The chunk's position in the structure
    pub chunk_location: ChunkCoordinate,
}

#[derive(Debug, Event)]
/// Sent whenever a chunk is unloaded from a structure
///
/// This event is NOT generated when a structure is despawned or when a chunk loses all its blocks or when a chunk with no blocks is unloaded.
///
/// This event's only current usecase is removing chunk's colliders when planet chunks are unloaded by a player moving away from them.
pub struct ChunkUnloadEvent {
    /// The chunk's entity. This will not have been despawned yet until after the Update system set.
    pub chunk_entity: Entity,
    /// The coordinates of the chunk in the structure
    pub coords: ChunkCoordinate,
    /// The structure's entity
    pub structure_entity: Entity,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChunkUnloadEvent>().register_type::<Chunk>();
}
