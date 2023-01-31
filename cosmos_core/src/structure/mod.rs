use bevy::utils::HashMap;
use bevy::{ecs::schedule::StateData, prelude::App};

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
use crate::registry::identifiable::Identifiable;
use crate::registry::Registry;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::utils::array_utils::flatten;
use bevy::prelude::{Component, Entity, EventWriter, GlobalTransform, Vec3};
use serde::{Deserialize, Serialize};

use self::block_health::block_destroyed_event::BlockDestroyedEvent;
use self::structure_block::StructureBlock;
use self::structure_iterator::{BlockIterator, ChunkIterator};

#[derive(Serialize, Deserialize, Component)]
pub struct Structure {
    #[serde(skip)]
    chunk_entities: Vec<Option<Entity>>,
    #[serde(skip)]
    chunk_entity_map: HashMap<Entity, usize>,
    #[serde(skip)]
    self_entity: Option<Entity>,

    chunks: Vec<Chunk>,
    width: usize,
    height: usize,
    length: usize,
}

impl Structure {
    pub fn new(width: usize, height: usize, length: usize, self_entity: Entity) -> Self {
        let mut chunks = Vec::with_capacity(width * height * length);

        for z in 0..length {
            for y in 0..height {
                for x in 0..width {
                    chunks.push(Chunk::new(x, y, z));
                }
            }
        }

        let mut chunk_entities = Vec::with_capacity(chunks.len());

        for _ in 0..(length * width * height) {
            chunk_entities.push(None);
        }

        Self {
            chunk_entities,
            self_entity: Some(self_entity),
            chunks,
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

    pub fn chunk_entity(&self, cx: usize, cy: usize, cz: usize) -> Entity {
        // If this fails, that means the chunk entity ids were not set before being used
        self.chunk_entities[flatten(cx, cy, cz, self.width, self.height)].unwrap()
    }

    pub fn set_chunk_entity(&mut self, cx: usize, cy: usize, cz: usize, entity: Entity) {
        if self.chunk_entities.is_empty() {
            for _ in 0..(self.width * self.height * self.length) {
                self.chunk_entities.push(None);
            }
        }

        let index = flatten(cx, cy, cz, self.width, self.height);

        self.chunk_entity_map.insert(entity, index);
        self.chunk_entities[index] = Some(entity);
    }

    pub fn chunk_from_entity(&self, entity: &Entity) -> Option<&Chunk> {
        if let Some(index) = self.chunk_entity_map.get(entity) {
            Some(&self.chunks[*index])
        } else {
            None
        }
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
    pub fn chunk_from_chunk_coordinates(&self, cx: usize, cy: usize, cz: usize) -> &Chunk {
        &self.chunks[flatten(cx, cy, cz, self.width, self.height)]
    }

    pub fn mut_chunk_from_chunk_coordinates(
        &mut self,
        cx: usize,
        cy: usize,
        cz: usize,
    ) -> &mut Chunk {
        &mut self.chunks[flatten(cx, cy, cz, self.width, self.height)]
    }

    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (5, 0, 0) => chunk @ 0, 0, 0\
    /// (32, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_at_block_coordinates(&self, x: usize, y: usize, z: usize) -> &Chunk {
        self.chunk_from_chunk_coordinates(
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
        )
    }

    fn mut_chunk_at_block_coordinates(&mut self, x: usize, y: usize, z: usize) -> &mut Chunk {
        &mut self.chunks[flatten(
            x / CHUNK_DIMENSIONS,
            y / CHUNK_DIMENSIONS,
            z / CHUNK_DIMENSIONS,
            self.width,
            self.height,
        )]
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

    pub fn block_id_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.chunk_at_block_coordinates(x, y, z).block_at(
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
        )
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

    pub fn chunks(&self) -> &Vec<Chunk> {
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

        self.mut_chunk_at_block_coordinates(x, y, z).set_block_at(
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
            block,
        );
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

        // add_vec(
        //     &body_position.translation(),
        //     &body_position
        //         .position
        //         .rotation
        //         .transform_vector(&),
        // )
    }

    pub fn block_world_position(
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
        self.chunks[i] = chunk;
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn all_chunks_iter(&self) -> ChunkIterator {
        ChunkIterator::new(
            0_i32,
            0_i32,
            0_i32,
            self.blocks_width() as i32 - 1,
            self.blocks_height() as i32 - 1,
            self.blocks_length() as i32 - 1,
            self,
        )
    }

    /// Iterate over blocks in a given range. Will skip over any out of bounds positions.
    /// Coordinates are inclusive
    pub fn chunk_iter(&self, start: (i32, i32, i32), end: (i32, i32, i32)) -> ChunkIterator {
        ChunkIterator::new(start.0, start.1, start.2, end.0, end.1, end.2, self)
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
            .get_block_health(
                bx % CHUNK_DIMENSIONS,
                by % CHUNK_DIMENSIONS,
                bz % CHUNK_DIMENSIONS,
                block_hardness,
            )
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
        let destroyed = self
            .mut_chunk_at_block_coordinates(bx, by, bz)
            .block_take_damage(
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
    }
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_game_state: T,
) {
    systems::register(app, post_loading_state, playing_game_state);
    ship::register(app, playing_game_state);
    events::register(app);
    loading::register(app);
    block_health::register(app);
    structure_block::register(app);
}
