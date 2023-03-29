use bevy::prelude::{App, CoreSet, DespawnRecursiveExt};
use bevy::reflect::Reflect;
use bevy::utils::{HashMap, HashSet};
use bevy_rapier3d::prelude::PhysicsWorld;

pub mod block_health;
pub mod chunk;
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
use crate::block::Block;
use crate::events::block_events::BlockChangedEvent;
use crate::netty::NoSendEntity;
use crate::physics::location::Location;
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::utils::array_utils::flatten;
use bevy::prelude::{
    BuildChildren, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform,
    IntoSystemConfig, PbrBundle, Query, States, Transform, Vec3,
};
use serde::{Deserialize, Serialize};

use self::block_health::block_destroyed_event::BlockDestroyedEvent;
use self::events::ChunkSetEvent;
use self::structure_block::StructureBlock;
use self::structure_iterator::{BlockIterator, ChunkIterator};

#[derive(Serialize, Deserialize, Component, Reflect, Debug)]
pub struct Structure {
    #[serde(skip)]
    chunk_entities: HashMap<usize, Entity>,
    #[serde(skip)]
    chunk_entity_map: HashMap<Entity, usize>,
    #[serde(skip)]
    self_entity: Option<Entity>,

    chunks: HashMap<usize, Chunk>,
    width: usize,
    height: usize,
    length: usize,
}

impl Structure {
    pub fn new(width: usize, height: usize, length: usize) -> Self {
        Self {
            chunk_entities: HashMap::default(),
            self_entity: None,
            chunks: HashMap::default(),
            width,
            height,
            length,
            chunk_entity_map: HashMap::default(),
        }
    }

    #[inline]
    pub fn chunks_width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn chunks_height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn chunks_length(&self) -> usize {
        self.length
    }

    #[inline]
    pub fn blocks_width(&self) -> usize {
        self.width * CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn blocks_height(&self) -> usize {
        self.height * CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn blocks_length(&self) -> usize {
        self.length * CHUNK_DIMENSIONS
    }

    /// Returns the entity for this chunk -- an empty chunk WILL NOT have an entity.
    ///
    /// If this returns none, that means the chunk entity was not set before being used.
    /// Maybe the chunk is empty or unloaded?
    pub fn chunk_entity(&self, cx: usize, cy: usize, cz: usize) -> Option<Entity> {
        let index = flatten(cx, cy, cz, self.width, self.height);

        self.chunk_entities.get(&index).copied()
    }

    pub fn set_chunk_entity(&mut self, cx: usize, cy: usize, cz: usize, entity: Entity) {
        let index = flatten(cx, cy, cz, self.width, self.height);

        self.chunk_entity_map.insert(entity, index);
        self.chunk_entities.insert(index, entity);
    }

    pub fn chunk_from_entity(&self, entity: &Entity) -> Option<&Chunk> {
        self.chunk_entity_map.get(entity).map(|x| &self.chunks[x])
    }

    pub fn block_coords_for_chunk(&self, chunk: &Chunk) -> (usize, usize, usize) {
        (
            CHUNK_DIMENSIONS * chunk.structure_x(),
            CHUNK_DIMENSIONS * chunk.structure_y(),
            CHUNK_DIMENSIONS * chunk.structure_z(),
        )
    }

    pub fn block_coords_for_chunk_block_coords(
        &self,
        chunk: &Chunk,
        block_coords: (usize, usize, usize),
    ) -> (usize, usize, usize) {
        (
            CHUNK_DIMENSIONS * chunk.structure_x() + block_coords.0,
            CHUNK_DIMENSIONS * chunk.structure_y() + block_coords.1,
            CHUNK_DIMENSIONS * chunk.structure_z() + block_coords.2,
        )
    }

    pub fn set_entity(&mut self, entity: Entity) {
        self.self_entity = Some(entity);
    }

    pub fn get_entity(&self) -> Option<Entity> {
        self.self_entity
    }

    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0
    ///
    /// Returns None for empty chunks - panics for chunks that are out of bounds
    pub fn chunk_from_chunk_coordinates(&self, cx: usize, cy: usize, cz: usize) -> Option<&Chunk> {
        assert!(
            cx < self.width && cy < self.height && cz < self.length,
            "{cx} < {} && {cy} < {} && {cz} < {} failed",
            self.width,
            self.height,
            self.length
        );

        self.chunks
            .get(&flatten(cx, cy, cz, self.width, self.height))
    }

    pub fn mut_chunk_from_chunk_coordinates(
        &mut self,
        cx: usize,
        cy: usize,
        cz: usize,
    ) -> Option<&mut Chunk> {
        self.chunks
            .get_mut(&flatten(cx, cy, cz, self.width, self.height))
    }

    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (5, 0, 0) => chunk @ 0, 0, 0\
    /// (32, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at_block_coordinates(&self, x: usize, y: usize, z: usize) -> Option<&Chunk> {
        self.chunk_from_chunk_coordinates(
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
        )
    }

    fn mut_chunk_at_block_coordinates(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
    ) -> Option<&mut Chunk> {
        self.mut_chunk_from_chunk_coordinates(
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
        )
    }

    pub fn is_within_blocks(&self, x: usize, y: usize, z: usize) -> bool {
        x < self.blocks_width() && y < self.blocks_height() && z < self.blocks_length()
    }

    pub fn has_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_id_at(x, y, z) != AIR_BLOCK_ID
    }

    /// # Arguments
    /// Coordinates relative to the structure's 0, 0, 0 position in the world mapped to block coordinates
    /// # Returns
    /// - Ok (x, y, z) of the block coordinates if the point is within the structure
    /// - Err(false) if one of the x/y/z coordinates are outside the structure in the negative direction
    /// - Err (true) if one of the x/y/z coordinates are outside the structure in the positive direction
    pub fn relative_coords_to_local_coords(
        &self,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<(usize, usize, usize), bool> {
        let xx = x + (self.blocks_width() as f32 / 2.0);
        let yy = y + (self.blocks_height() as f32 / 2.0);
        let zz = z + (self.blocks_length() as f32 / 2.0);

        if xx >= 0.0 && yy >= 0.0 && zz >= 0.0 {
            let (xxx, yyy, zzz) = (xx as usize, yy as usize, zz as usize);
            if self.is_within_blocks(xxx, yyy, zzz) {
                return Ok((xxx, yyy, zzz));
            }
            return Err(true);
        }
        Err(false)
    }

    /// If the chunk is loaded/non-empty, returns the block at that coordinate.
    /// Otherwise, returns AIR_BLOCK_ID
    pub fn block_id_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.chunk_at_block_coordinates(x, y, z)
            .map(|chunk| {
                chunk.block_at(
                    x % CHUNK_DIMENSIONS,
                    y % CHUNK_DIMENSIONS,
                    z % CHUNK_DIMENSIONS,
                )
            })
            .unwrap_or(AIR_BLOCK_ID)
    }

    pub fn block_at<'a>(
        &'a self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &'a Registry<Block>,
    ) -> &'a Block {
        let id = self.block_id_at(x, y, z);
        blocks.from_numeric_id(id)
    }

    pub fn chunks(&self) -> &HashMap<usize, Chunk> {
        &self.chunks
    }

    pub fn remove_block_at(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.set_block_at(
            x,
            y,
            z,
            blocks.from_numeric_id(AIR_BLOCK_ID),
            blocks,
            event_writer,
        )
    }

    fn create_chunk_at(&mut self, cx: usize, cy: usize, cz: usize) -> &mut Chunk {
        let index = flatten(cx, cy, cz, self.width, self.height);

        self.chunks.insert(index, Chunk::new(cx, cy, cz));

        self.chunks.get_mut(&index).unwrap()
    }

    /// Removes the chunk at the given coordinate -- does NOT remove the chunk entity
    fn unload_chunk(&mut self, cx: usize, cy: usize, cz: usize) {
        self.chunks
            .remove(&flatten(cx, cy, cz, self.width, self.height));
    }

    pub fn set_block_at(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        block: &Block,
        blocks: &Registry<Block>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        let old_block = self.block_id_at(x, y, z);
        if blocks.from_numeric_id(old_block) == block {
            return;
        }

        if let Some(self_entity) = self.self_entity {
            if let Some(event_writer) = event_writer {
                event_writer.send(BlockChangedEvent {
                    new_block: block.id(),
                    old_block,
                    structure_entity: self_entity,
                    block: StructureBlock::new(x, y, z),
                });
            }
        }

        let (bx, by, bz) = (
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
        );

        let (cx, cy, cz) = (
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
        );

        if let Some(chunk) = self.mut_chunk_at_block_coordinates(x, y, z) {
            chunk.set_block_at(bx, by, bz, block);

            if chunk.is_empty() {
                self.unload_chunk(cx, cy, cz);
            }
        } else if block.id() != AIR_BLOCK_ID {
            let chunk = self.create_chunk_at(cx, cy, cz);
            chunk.set_block_at(bx, by, bz, block);
        }
    }

    pub fn chunk_relative_position(&self, x: usize, y: usize, z: usize) -> Vec3 {
        let xoff = (self.width as f32 - 1.0) / 2.0;
        let yoff = (self.height as f32 - 1.0) / 2.0;
        let zoff = (self.length as f32 - 1.0) / 2.0;

        let xx = CHUNK_DIMENSIONS as f32 * (x as f32 - xoff);
        let yy = CHUNK_DIMENSIONS as f32 * (y as f32 - yoff);
        let zz = CHUNK_DIMENSIONS as f32 * (z as f32 - zoff);

        Vec3::new(xx, yy, zz)
    }

    pub fn block_relative_position(&self, x: usize, y: usize, z: usize) -> Vec3 {
        let xoff = self.blocks_width() as f32 / 2.0;
        let yoff = self.blocks_height() as f32 / 2.0;
        let zoff = self.blocks_length() as f32 / 2.0;

        let xx = x as f32 - xoff;
        let yy = y as f32 - yoff;
        let zz = z as f32 - zoff;

        Vec3::new(xx + 0.5, yy + 0.5, zz + 0.5)
    }

    pub fn chunk_world_position(
        &self,
        x: usize,
        y: usize,
        z: usize,
        body_position: &GlobalTransform,
    ) -> Vec3 {
        body_position.translation()
            + body_position
                .affine()
                .matrix3
                .mul_vec3(self.chunk_relative_position(x, y, z))
    }

    pub fn block_world_position(
        &self,
        x: usize,
        y: usize,
        z: usize,
        body_position: &GlobalTransform,
        this_location: &Location,
    ) -> Location {
        *this_location
            + body_position
                .affine()
                .matrix3
                .mul_vec3(self.block_relative_position(x, y, z))
    }

    pub fn set_chunk(&mut self, chunk: Chunk) {
        let i = flatten(
            chunk.structure_x(),
            chunk.structure_y(),
            chunk.structure_z(),
            self.width,
            self.height,
        );
        self.chunks.insert(i, chunk);
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    ///
    /// If include_empty is enabled, the value iterated over may be None OR Some(chunk).
    /// If include_empty is disabled, the value iterated over may ONLY BE Some(chunk).
    pub fn all_chunks_iter(&self, include_empty: bool) -> ChunkIterator {
        ChunkIterator::new(
            0_i32,
            0_i32,
            0_i32,
            self.blocks_width() as i32 - 1,
            self.blocks_height() as i32 - 1,
            self.blocks_length() as i32 - 1,
            self,
            include_empty,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn chunk_iter(
        &self,
        start: (i32, i32, i32),
        end: (i32, i32, i32),
        include_empty: bool,
    ) -> ChunkIterator {
        ChunkIterator::new(
            start.0,
            start.1,
            start.2,
            end.0,
            end.1,
            end.2,
            self,
            include_empty,
        )
    }

    /// Will fail assertion if chunk positions are out of bounds
    pub fn block_iter_for_chunk(
        &self,
        (cx, cy, cz): (usize, usize, usize),
        include_air: bool,
    ) -> BlockIterator {
        assert!(cx < self.width && cy < self.height && cz < self.length);

        BlockIterator::new(
            (cx * CHUNK_DIMENSIONS) as i32,
            (cy * CHUNK_DIMENSIONS) as i32,
            (cz * CHUNK_DIMENSIONS) as i32,
            ((cx + 1) * CHUNK_DIMENSIONS) as i32 - 1,
            ((cy + 1) * CHUNK_DIMENSIONS) as i32 - 1,
            ((cz + 1) * CHUNK_DIMENSIONS) as i32 - 1,
            include_air,
            self,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn all_blocks_iter(&self, include_air: bool) -> BlockIterator {
        BlockIterator::new(
            0_i32,
            0_i32,
            0_i32,
            self.blocks_width() as i32 - 1,
            self.blocks_height() as i32 - 1,
            self.blocks_length() as i32 - 1,
            include_air,
            self,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn block_iter(
        &self,
        start: (i32, i32, i32),
        end: (i32, i32, i32),
        include_air: bool,
    ) -> BlockIterator {
        BlockIterator::new(
            start.0,
            start.1,
            start.2,
            end.0,
            end.1,
            end.2,
            include_air,
            self,
        )
    }

    /// Gets the block's health at that given coordinate
    /// - x/y/z: block coordinate
    /// - block_hardness: The hardness for the block at those coordinates
    pub fn get_block_health(
        &mut self,
        bx: usize,
        by: usize,
        bz: usize,
        block_hardness: &BlockHardness,
    ) -> f32 {
        self.chunk_at_block_coordinates(bx, by, bz)
            .map(|c| {
                c.get_block_health(
                    bx % CHUNK_DIMENSIONS,
                    by % CHUNK_DIMENSIONS,
                    bz % CHUNK_DIMENSIONS,
                    block_hardness,
                )
            })
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
        bx: usize,
        by: usize,
        bz: usize,
        block_hardness: &BlockHardness,
        amount: f32,
        event_writer: Option<&mut EventWriter<BlockDestroyedEvent>>,
    ) -> bool {
        if let Some(chunk) = self.mut_chunk_at_block_coordinates(bx, by, bz) {
            let destroyed = chunk.block_take_damage(
                bx % CHUNK_DIMENSIONS,
                by % CHUNK_DIMENSIONS,
                bz % CHUNK_DIMENSIONS,
                block_hardness,
                amount,
            );

            if destroyed {
                if let Some(structure_entity) = self.get_entity() {
                    if let Some(event_writer) = event_writer {
                        event_writer.send(BlockDestroyedEvent {
                            block: StructureBlock::new(bx, by, bz),
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
}

#[derive(Debug)]
pub struct ChunkInitEvent {
    /// The entity of the structure this is a part of
    pub structure_entity: Entity,
    /// Chunk's coordinate in the structure
    pub x: usize,
    /// Chunk's coordinate in the structure    
    pub y: usize,
    /// Chunk's coordinate in the structure    
    pub z: usize,
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

        let (cx, cy, cz) = bce.block.chunk_coords();

        if structure.chunk_from_chunk_coordinates(cx, cy, cz).is_none() {
            if let Some(chunk_entity) = structure.chunk_entity(cx, cy, cz) {
                commands.entity(chunk_entity).despawn_recursive();

                let (width, height) = (structure.width, structure.height);

                structure
                    .chunk_entities
                    .remove(&flatten(cx, cy, cz, width, height));
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
        s_chunks.insert((
            ev.structure_entity,
            (
                ev.block.x / CHUNK_DIMENSIONS,
                ev.block.y / CHUNK_DIMENSIONS,
                ev.block.z / CHUNK_DIMENSIONS,
            ),
        ));
    }

    for ev in chunk_init_reader.iter() {
        s_chunks.insert((ev.structure_entity, (ev.x, ev.y, ev.z)));
        chunk_set_events.insert(ChunkSetEvent {
            structure_entity: ev.structure_entity,
            x: ev.x,
            y: ev.y,
            z: ev.z,
        });
    }

    for (structure_entity, (x, y, z)) in s_chunks {
        if let Ok((mut structure, body_world)) = structure_query.get_mut(structure_entity) {
            if let Some(chunk) = structure.chunk_from_chunk_coordinates(x, y, z) {
                if !chunk.is_empty() && structure.chunk_entity(x, y, z).is_none() {
                    let mut entity_cmds = commands.spawn((
                        PbrBundle {
                            transform: Transform::from_translation(
                                structure.chunk_relative_position(x, y, z),
                            ),
                            ..Default::default()
                        },
                        NoSendEntity,
                    ));

                    if let Some(bw) = body_world {
                        entity_cmds.insert(*bw);
                    }

                    let entity = entity_cmds.id();

                    commands.entity(structure_entity).add_child(entity);

                    structure.set_chunk_entity(x, y, z, entity);

                    chunk_set_events.insert(ChunkSetEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                    });
                }
            }
        }
    }

    for ev in chunk_set_events {
        chunk_set_event_writer.send(ev);
    }
}

pub(crate) fn register<T: States + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_game_state: T,
) {
    app.register_type::<Structure>()
        .register_type::<Chunk>()
        .add_event::<ChunkInitEvent>();

    systems::register(app, post_loading_state, playing_game_state);
    ship::register(app, playing_game_state);
    events::register(app);
    loading::register(app);
    block_health::register(app);
    structure_block::register(app);

    app.add_system(add_chunks_system.in_base_set(CoreSet::PreUpdate))
        .add_system(remove_empty_chunks.after(add_chunks_system));
}
