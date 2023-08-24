use bevy::{
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use super::{block_storage::BlockStorage, chunk::CHUNK_DIMENSIONS, coordinates::CoordinateType};

#[derive(Debug, Reflect, Serialize, Deserialize, DerefMut, Deref)]
pub struct LodChunk(BlockStorage);

impl LodChunk {
    pub fn new() -> Self {
        Self(BlockStorage::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS))
    }
}
