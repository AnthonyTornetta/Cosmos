//! Represents a fixed region of blocks.
//!
//! These blocks can be updated.

use bevy::ecs::query::{QueryData, QueryFilter, QueryItem, ROQueryItem, With};
use bevy::ecs::system::{Commands, Query};
use bevy::hierarchy::{BuildChildren, DespawnRecursiveExt};
use bevy::log::{error, info};
use bevy::prelude::{App, Component, Entity, Event, Vec3};
use bevy::reflect::Reflect;
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};

use crate::block::data::{BlockData, BlockDataIdentifier};
use crate::block::{Block, BlockFace, BlockRotation, BlockSubRotation};
use crate::ecs::NeedsDespawned;
use crate::events::block_events::{BlockDataChangedEvent, BlockDataSystemParams};
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;

use super::block_health::BlockHealth;
use super::block_storage::{BlockStorage, BlockStorer};
use super::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType, UnboundCoordinateType};
use super::structure_block::StructureBlock;

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
    /// Removed block data should be stored here to be depsawned later
    ///
    /// Useful to not have every method require arguments to interact with the bevy ECS
    #[serde(skip)]
    removed_block_data: Vec<Entity>,
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
    fn block_rotation(&self, coords: ChunkBlockCoordinate) -> BlockRotation {
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
    fn set_block_at(&mut self, coords: ChunkBlockCoordinate, b: &Block, block_rotation: BlockRotation) {
        if b.id() == self.block_at(coords) {
            return;
        }

        // if let Some(bd_ent) = self.block_data.remove(&coords) {
        //     self.removed_block_data.push(bd_ent);
        // }

        self.block_storage.set_block_at(coords, b, block_rotation)
    }

    #[inline(always)]
    fn set_block_at_from_id(&mut self, coords: ChunkBlockCoordinate, id: u16, block_rotation: BlockRotation) {
        if id == self.block_at(coords) {
            return;
        }

        // if let Some(bd_ent) = self.block_data.remove(&coords) {
        //     self.removed_block_data.push(bd_ent);
        // }

        self.block_storage.set_block_at_from_id(coords, id, block_rotation)
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
            removed_block_data: Default::default(),
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

    /// Sets the block data entity for these coordinates.
    ///
    /// This will NOT despawn the entity, nor mark it for deletion.
    pub fn set_block_data_entity(&mut self, coords: ChunkBlockCoordinate, entity: Option<Entity>) {
        if let Some(e) = entity {
            self.block_data.insert(coords, e);
        } else {
            self.block_data.remove(&coords);
        }
    }

    /// Despawns any block data that is no longer used by any blocks. This should be called every frame
    /// for general cleanup and avoid systems executing on dead block-data.
    pub fn despawn_dead_block_data(&mut self, bs_commands: &mut BlockDataSystemParams) {
        for ent in std::mem::take(&mut self.removed_block_data) {
            // Don't send block data changed event here, since the only way this happens is if the block itself is changed
            // to another block.

            if let Some(ecmds) = bs_commands.commands.get_entity(ent) {
                ecmds.despawn_recursive();
            }
        }
    }

    /// Inserts data into the block here. Returns the entity that stores this block's data.
    pub fn insert_block_data<T: Component>(
        &mut self,
        coords: ChunkBlockCoordinate,
        chunk_entity: Entity,
        structure_entity: Entity,
        data: T,
        system_params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Entity {
        if let Some(data_ent) = self.block_data(coords) {
            info!("Entity existed - {data_ent:?}");
            if let Ok(mut bd) = q_block_data.get_mut(data_ent) {
                if !q_data.contains(data_ent) {
                    bd.increment();
                }
            } else {
                system_params.commands.entity(data_ent).log_components();
                error!("Block data entity missing BlockData component!");
            }

            system_params.commands.entity(data_ent).insert(data);

            system_params.ev_writer.send(BlockDataChangedEvent {
                block_data_entity: Some(data_ent),
                block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                structure_entity,
            });

            data_ent
        } else {
            let data_ent = system_params
                .commands
                .spawn((
                    data,
                    BlockData {
                        data_count: 1,
                        identifier: BlockDataIdentifier {
                            block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                            structure_entity,
                        },
                    },
                ))
                .set_parent(chunk_entity)
                .id();

            info!("Created new data entity - {data_ent:?}");

            self.block_data.insert(coords, data_ent);

            system_params.ev_writer.send(BlockDataChangedEvent {
                block_data_entity: Some(data_ent),
                block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                structure_entity,
            });

            data_ent
        }
    }

    /// Gets or creates the block data entity for the block here.
    pub fn get_or_create_block_data(
        &mut self,
        coords: ChunkBlockCoordinate,
        chunk_entity: Entity,
        structure_entity: Entity,
        commands: &mut Commands,
    ) -> Option<Entity> {
        if let Some(data_ent) = self.block_data(coords) {
            return Some(data_ent);
        }

        let data_ent = commands
            .spawn((BlockData {
                data_count: 0,
                identifier: BlockDataIdentifier {
                    block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                    structure_entity,
                },
            },))
            .set_parent(chunk_entity)
            .id();

        self.block_data.insert(coords, data_ent);

        Some(data_ent)
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
        coords: ChunkBlockCoordinate,
        chunk_entity: Entity,
        structure_entity: Entity,
        create_data_closure: F,
        system_params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Entity
    where
        F: FnOnce(Entity) -> T,
    {
        let data_ent = if let Some(data_ent) = self.block_data(coords) {
            let data = create_data_closure(data_ent);

            let Ok(mut bd) = q_block_data.get_mut(data_ent) else {
                panic!("Block data entity missing BlockData component!");
            };

            if !q_data.contains(data_ent) {
                bd.increment();
            }

            system_params.commands.entity(data_ent).insert(data);

            data_ent
        } else {
            let mut ecmds = system_params.commands.spawn(BlockData {
                data_count: 1,
                identifier: BlockDataIdentifier {
                    block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                    structure_entity,
                },
            });

            ecmds.set_parent(chunk_entity);

            let data_ent = ecmds.id();

            let data = create_data_closure(data_ent);

            ecmds.insert(data);

            self.block_data.insert(coords, data_ent);

            data_ent
        };

        system_params.ev_writer.send(BlockDataChangedEvent {
            block_data_entity: Some(data_ent),
            block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
            structure_entity,
        });

        data_ent
    }

    /// Queries this block's data. Returns `None` if the requested query failed or if no block data exists for this block.
    pub fn query_block_data<'a, Q, F>(&'a self, coords: ChunkBlockCoordinate, query: &'a Query<Q, F>) -> Option<ROQueryItem<'a, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        let data_ent = self.block_data(coords)?;

        query.get(data_ent).ok()
    }

    /// Queries this block's data mutibly. Returns `None` if the requested query failed or if no block data exists for this block.
    pub fn query_block_data_mut<'a, Q, F>(
        &'a self,
        coords: ChunkBlockCoordinate,
        query: &'a mut Query<Q, F>,
        block_system_params: &mut BlockDataSystemParams,
        structure_entity: Entity,
    ) -> Option<QueryItem<'a, Q>>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        let data_ent = self.block_data(coords)?;

        match query.get_mut(data_ent) {
            Ok(result) => {
                block_system_params.ev_writer.send(BlockDataChangedEvent {
                    block_data_entity: Some(data_ent),
                    block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                    structure_entity,
                });

                Some(result)
            }
            Err(_) => None,
        }
    }

    /// Removes this type of data from the block here. Returns the entity that stores this blocks data
    /// if it will still exist.
    pub fn remove_block_data<T: Component>(
        &mut self,
        structure_entity: Entity,
        coords: ChunkBlockCoordinate,
        system_params: &mut BlockDataSystemParams,
        q_block_data: &mut Query<&mut BlockData>,
        q_data: &Query<(), With<T>>,
    ) -> Option<Entity> {
        let ent = self.block_data(coords)?;

        let Ok(mut bd) = q_block_data.get_mut(ent) else {
            panic!("Block data entity missing BlockData component!");
        };

        if q_data.contains(ent) {
            bd.decrement();
        }

        if bd.is_empty() {
            system_params.commands.entity(ent).insert(NeedsDespawned);
            self.block_data.remove(&coords);

            info!("Despawning bd entity - {ent:?}");

            system_params.ev_writer.send(BlockDataChangedEvent {
                block_data_entity: None,
                block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                structure_entity,
            });

            None
        } else {
            system_params.commands.entity(ent).remove::<T>();

            system_params.ev_writer.send(BlockDataChangedEvent {
                block_data_entity: Some(ent),
                block: StructureBlock::new(self.chunk_coordinates().first_structure_block() + coords),
                structure_entity,
            });

            Some(ent)
        }
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
    /// This will return which BlockFace represents the UP direction (no rotation is BlockFace::Top, BlockSubRotation::None)
    pub fn get_rotation(&self) -> BlockRotation {
        let block_up = BlockFace::from_index((self.0 & 0b111) as usize);
        let sub_rotation = BlockSubRotation::from_index(((self.0 >> 3) & 0b11) as usize);

        BlockRotation { block_up, sub_rotation }
    }

    /// Sets the rotation data
    pub fn set_rotation(&mut self, rotation: BlockRotation) {
        self.0 = self.0 & !0b11111 | (rotation.block_up.index() as u8 | (rotation.sub_rotation.index() << 3) as u8);
    }
}

/// This entity represents a chunk stored within the structure
///
/// For performance reasons, the chunk itself is stored within the [`Structure`](super::Structure)
/// component in a structure entity.
/// To access this structure entity, use this component's [`structure_entity`](Self::structure_entity) field.
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

// #[cfg(test)]
// mod tests {
//     use crate::block::{BlockFace, BlockRotation, BlockSubRotation};

//     #[test]
//     fn test_quaternion() {
//         let rot = BlockRotation {
//             block_up: BlockFace::Right,
//             sub_rotation: BlockSubRotation::
//         }
//     }
// }
