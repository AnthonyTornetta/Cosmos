use crate::block::block::Block;
use crate::block::blocks::{AIR, block_from_id};
use serde::{Serialize, Deserialize};

pub const CHUNK_DIMENSIONS: usize = 32;
const N_BLOCKS: usize = CHUNK_DIMENSIONS * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS;

#[derive(Serialize, Deserialize)]
pub struct Chunk
{
    x: usize,
    y: usize,
    z: usize,

    #[serde(with = "serde_arrays")]
    blocks: [u16; N_BLOCKS]
}

impl Chunk
{
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self {
            x, y, z,
            blocks: [0; N_BLOCKS]
        }
    }

    #[inline]
    pub fn structure_x(&self) -> usize {
        self.x
    }

    #[inline]
    pub fn structure_y(&self) -> usize {
        self.y
    }

    #[inline]
    pub fn structure_z(&self) -> usize {
        self.z
    }

    pub fn set_block_at(&mut self, x: usize, y: usize, z: usize, b: &Block) {
        self.blocks[z * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS + y * CHUNK_DIMENSIONS + x] = b.id();
    }

    pub fn has_see_through_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_at(x, y, z).is_see_through()
    }

    pub fn has_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        *self.block_at(x, y, z) != *AIR
    }

    pub fn block_at(&self, x: usize, y: usize, z: usize) -> &'static Block {
        block_from_id(self.blocks[z * CHUNK_DIMENSIONS * CHUNK_DIMENSIONS + y * CHUNK_DIMENSIONS + x])
    }

    pub fn has_full_block_at(&self, x: usize, y: usize, z: usize) -> bool {
        self.block_at(x, y, z).is_full()
    }
}
