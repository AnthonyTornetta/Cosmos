use std::num::NonZeroU8;

use crate::block::block::Block;
use crate::block::blocks::{Blocks, AIR_BLOCK_ID};
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::utils::array_utils::flatten;
use crate::utils::vec_math::add_vec;
use bevy::prelude::{Component, Entity, EventWriter, Res};
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::rapier::prelude::RigidBodyPosition;
use serde::{Deserialize, Serialize};

pub struct StructureCreated {
    pub entity: Entity,
}

pub struct ChunkSetEvent {
    pub structure_entity: Entity,
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

#[derive(Serialize, Deserialize, Component)]
pub struct Structure {
    #[serde(skip)]
    chunk_entities: Vec<Option<Entity>>,
    #[serde(skip)]
    self_entity: Option<Entity>,

    chunks: Vec<Chunk>,
    width: usize,
    height: usize,
    length: usize,
}

pub struct BlockChangedEvent {
    pub block: StructureBlock,
    pub structure_entity: Entity,
    pub old_block: u16,
    pub new_block: u16,
}

pub struct StructureBlock {
    x: usize,
    y: usize,
    z: usize,
}

impl StructureBlock {
    #[inline]
    pub fn x(&self) -> usize {
        self.x
    }
    #[inline]
    pub fn y(&self) -> usize {
        self.y
    }
    #[inline]
    pub fn z(&self) -> usize {
        self.z
    }

    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn block(&self, structure: &Structure) -> u16 {
        structure.block_at(self.x, self.y, self.z)
    }

    #[inline]
    pub fn chunk_coord_x(&self) -> usize {
        self.x / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coord_y(&self) -> usize {
        self.y / CHUNK_DIMENSIONS
    }

    #[inline]
    pub fn chunk_coord_z(&self) -> usize {
        self.z / CHUNK_DIMENSIONS
    }
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
        self.chunk_entities[flatten(cx, cy, cz, self.width, self.height)]
            .unwrap()
            .clone()
    }

    pub fn set_chunk_entity(&mut self, cx: usize, cy: usize, cz: usize, entity: Entity) {
        if self.chunk_entities.len() == 0 {
            for _ in 0..(self.width * self.height * self.length) {
                self.chunk_entities.push(None);
            }
        }
        self.chunk_entities[flatten(cx, cy, cz, self.width, self.height)] = Some(entity);
    }

    pub fn set_entity(&mut self, entity: Entity) {
        self.self_entity = Some(entity);
    }

    pub fn get_entity(&self) -> Option<Entity> {
        self.self_entity.clone()
    }

    /// (0, 0, 0) => chunk @ 0, 0, 0\
    /// (1, 0, 0) => chunk @ 1, 0, 0
    pub fn chunk_from_chunk_coordinates(&self, cx: usize, cy: usize, cz: usize) -> &Chunk {
        &self.chunks[flatten(cx, cy, cz, self.width, self.height)]
    }

    fn mut_chunk_from_chunk_coordinates(&mut self, cx: usize, cy: usize, cz: usize) -> &mut Chunk {
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
        self.block_at(x, y, z) != AIR_BLOCK_ID
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
        // replace the + 0.5 with .round() at some point to make it a bit cleaner
        let xx = x + (self.blocks_width() as f32 / 2.0) + 0.5;
        let yy = y + (self.blocks_height() as f32 / 2.0) + 0.5;
        let zz = z + (self.blocks_length() as f32 / 2.0) + 0.5;

        if xx >= 0.0 && yy >= 0.0 && zz >= 0.0 {
            let (xxx, yyy, zzz) = (xx as usize, yy as usize, zz as usize);
            if self.is_within_blocks(xxx, yyy, zzz) {
                return Ok((xxx, yyy, zzz));
            }
            return Err(true);
        }
        return Err(false);
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> u16 {
        self.chunk_at_block_coordinates(x, y, z).block_at(
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
        )
    }

    pub fn chunks(&self) -> &Vec<Chunk> {
        &self.chunks
    }

    pub fn remove_block_at(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        blocks: &Res<Blocks>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        self.set_block_at(
            x,
            y,
            z,
            blocks.block_from_numeric_id(AIR_BLOCK_ID),
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
        blocks: &Res<Blocks>,
        event_writer: Option<&mut EventWriter<BlockChangedEvent>>,
    ) {
        let old_block = self.block_at(x, y, z);
        if blocks.block_from_numeric_id(old_block) == block {
            return;
        }

        if self.self_entity.is_some() && event_writer.is_some() {
            event_writer.unwrap().send(BlockChangedEvent {
                new_block: block.id(),
                old_block,
                structure_entity: self.self_entity.unwrap().clone(),
                block: StructureBlock::new(x, y, z),
            });
        }

        self.mut_chunk_at_block_coordinates(x, y, z).set_block_at(
            x % CHUNK_DIMENSIONS,
            y % CHUNK_DIMENSIONS,
            z % CHUNK_DIMENSIONS,
            block,
        );
    }

    pub fn chunk_relative_position(&self, x: usize, y: usize, z: usize) -> Vector3<f32> {
        let xoff = self.width as f32 / 2.0 * CHUNK_DIMENSIONS as f32;
        let yoff = self.height as f32 / 2.0 * CHUNK_DIMENSIONS as f32;
        let zoff = self.length as f32 / 2.0 * CHUNK_DIMENSIONS as f32;

        let xx = x as f32 * CHUNK_DIMENSIONS as f32 - xoff;
        let yy = y as f32 * CHUNK_DIMENSIONS as f32 - yoff;
        let zz = z as f32 * CHUNK_DIMENSIONS as f32 - zoff;

        Vector3::new(xx, yy, zz)
    }

    pub fn chunk_world_position(
        &self,
        x: usize,
        y: usize,
        z: usize,
        body_position: &RigidBodyPosition,
    ) -> Vector3<f32> {
        add_vec(
            &body_position.position.translation.vector,
            &body_position
                .position
                .rotation
                .transform_vector(&self.chunk_relative_position(x, y, z)),
        )
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
}
