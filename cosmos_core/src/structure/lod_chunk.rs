use bevy::{
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use super::{block_storage::BlockStorage, chunk::CHUNK_DIMENSIONS, coordinates::CoordinateType};

#[derive(Debug, Reflect, Serialize, Deserialize, DerefMut, Deref)]
pub struct LodChunk {
    // N chunks this contains = scale^2
    scale: CoordinateType,
    #[deref]
    block_storage: BlockStorage,
}

impl LodChunk {
    pub fn new(scale: CoordinateType) -> Self {
        Self {
            scale,
            block_storage: BlockStorage::new(CHUNK_DIMENSIONS, CHUNK_DIMENSIONS, CHUNK_DIMENSIONS),
        }
    }

    /// Gets the scale of this chunk
    pub fn scale(&self) -> CoordinateType {
        self.scale
    }
}
