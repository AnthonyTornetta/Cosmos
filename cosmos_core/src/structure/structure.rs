use std::cell::RefCell;
use std::rc::{Rc};
use std::sync::{Arc, Mutex};
use bevy_rapier3d::na::{Vector3};
use bevy_rapier3d::rapier::prelude::RigidBodyPosition;
use crate::block::block::Block;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::structure_listener::StructureListener;
use crate::utils::array_utils::flatten;
use crate::utils::vec_math::add_vec;
use serde::{Serialize, Deserialize};
use bevy::prelude::{Component, Entity};

#[derive(Serialize, Deserialize, Component)]
pub struct Structure
{
    #[serde(skip)]
    listeners: Vec<Arc<Mutex<dyn StructureListener>>>,

    chunks: Vec<Chunk>,
    width: usize,
    height: usize,
    length: usize
}

pub struct BlockChangedEvent {
    block: StructureBlock,
    structure_entity: Entity,
    old_block: &'static Block,
    new_block: &'static Block
}

pub struct StructureBlock {
    x: usize,
    y: usize,
    z: usize
}

impl StructureBlock {
    #[inline]
    pub fn x(&self) -> usize { self.x }
    #[inline]
    pub fn y(&self) -> usize { self.x }
    #[inline]
    pub fn z(&self) -> usize { self.x }

    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self {
            x, y, z
        }
    }

    #[inline]
    pub fn block(&self, structure: &Structure) -> &'static Block {
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
    pub fn new(width: usize, height: usize, length: usize) -> Self {
        let mut chunks = Vec::with_capacity(width * height * length);

        for z in 0..length
        {
            for y in 0..height
            {
                for x in 0..width
                {
                    chunks.push(Chunk::new(x, y, z));
                }
            }
        }

        Self {
            chunks,
            listeners: Vec::new(),
            width, height, length
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn length(&self) -> usize {
        self.length
    }

    pub fn add_structure_listener(&mut self, listener: Arc<Mutex<dyn StructureListener>>) {
        self.listeners.push(listener);
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
        self.chunk_from_chunk_coordinates(x / CHUNK_DIMENSIONS, y / CHUNK_DIMENSIONS, z / CHUNK_DIMENSIONS)
    }

    fn mut_chunk_at_block_coordinates(&mut self, x: usize, y: usize, z: usize) -> &mut Chunk {
        &mut self.chunks[flatten(x / CHUNK_DIMENSIONS, y / CHUNK_DIMENSIONS, z / CHUNK_DIMENSIONS, self.width, self.height)]
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> &'static Block {
        self.chunk_at_block_coordinates(x, y, z)
            .block_at(x % CHUNK_DIMENSIONS, y % CHUNK_DIMENSIONS, z % CHUNK_DIMENSIONS)
    }

    pub fn set_block_at(&mut self, x: usize, y: usize, z: usize, block: &'static Block) {
        if self.block_at(x, y, z) == block {
            return;
        }

        let struct_block = StructureBlock::new(x, y, z);

        for listener in &self.listeners {
            listener.borrow_mut().notify_block_update(self, &struct_block, block);
        }

        self.mut_chunk_at_block_coordinates(x, y, z)
            .set_block_at(x % CHUNK_DIMENSIONS, y % CHUNK_DIMENSIONS, z % CHUNK_DIMENSIONS, block);
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

    pub fn chunk_world_position(&self, x: usize, y: usize, z: usize, body_position: &RigidBodyPosition) -> Vector3<f32> {
        add_vec(&body_position.position.translation.vector,
                &body_position.position.rotation.transform_vector(&self.chunk_relative_position(x, y, z)))
    }
}