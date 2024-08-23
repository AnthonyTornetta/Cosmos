//! Ways of displaying reduced-detail versions of dynamic structures

use std::sync::{Arc, Mutex};

use bevy::prelude::Component;
use serde::{Deserialize, Serialize};

use crate::block::blocks::AIR_BLOCK_ID;

use super::{
    block_storage::BlockStorer,
    chunk::CHUNK_DIMENSIONS,
    coordinates::CoordinateType,
    lod_chunk::{LodBlockSubScale, LodChunk},
    prelude::{BlockCoordinate, ChunkBlockCoordinate},
};

#[derive(Debug, Clone, Component)]
/// Represents a reduced-detail version of a planet
pub struct LodComponent(pub Arc<Mutex<Lod>>);

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents a reduced-detail version of a planet
pub enum Lod {
    /// No Lod here - this means there should be an actual chunk here
    None,
    /// Represents a single chunk of blocks at any scale.
    Single(Box<LodChunk>, bool),
    /// Breaks a single cube into 8 sub-cubes.
    ///
    /// The indicies of each cube follow a clockwise direction starting on the bottom-left-back
    ///
    /// ```txt
    ///    +-----------+
    ///   /  5    6   /|
    ///  /  4    7   / |
    /// +-----------+  |
    /// |           |  |  
    /// |           |  +
    /// |   1    2  | /
    /// |  0    3   |/
    /// +-----------+
    /// ```
    Children(Box<[Self; 8]>),
}

#[derive(Serialize, Deserialize, Component, Debug, Clone)]
/// Represents a change in the reduced-detail version of a planet
pub enum LodDelta {
    /// Keep the current version of the lod
    NoChange,
    /// No Lod here - this means there should be an actual chunk here
    None,
    /// Represents a single chunk of blocks at any scale.
    Single(Box<LodChunk>),
    /// Breaks a single cube into 8 sub-cubes.
    ///
    /// The indicies of each cube follow a clockwise direction starting on the bottom-left-back
    ///
    /// ```txt
    ///    +-----------+
    ///   /  5    6   /|
    ///  /  4    7   / |
    /// +-----------+  |
    /// |           |  |  
    /// |           |  +
    /// |   1    2  | /
    /// |  0    3   |/
    /// +-----------+
    /// ```
    Children(Box<[Self; 8]>),
}

impl Lod {
    /// Returns true if there is a non-air block at these coords in this LOD representation.
    pub fn has_block_at(&self, coords: BlockCoordinate, root_scale: CoordinateType) -> bool {
        self.block_id_at_and_scale(coords, root_scale).0 != AIR_BLOCK_ID
    }

    /// Returns the block at these coords in this LOD representation.
    pub fn block_id_at_and_scale(&self, coords: BlockCoordinate, root_scale: CoordinateType) -> (u16, LodBlockSubScale) {
        let scale = root_scale;
        match self {
            Lod::None => (AIR_BLOCK_ID, LodBlockSubScale::default()),
            Lod::Single(lod, _) => {
                let c = BlockCoordinate::new(coords.x / scale, coords.y / scale, coords.z / scale);

                if let Ok(chunk_block_coord) = ChunkBlockCoordinate::try_from(c) {
                    (lod.block_at(chunk_block_coord), lod.block_scale(chunk_block_coord))
                } else {
                    (AIR_BLOCK_ID, LodBlockSubScale::default())
                }
            }
            Lod::Children(children) => {
                let s2 = (scale * CHUNK_DIMENSIONS) / 2;

                let (idx, coords) = match (coords.x < s2, coords.y < s2, coords.z < s2) {
                    (true, true, true) => (0, coords),
                    (true, true, false) => (1, BlockCoordinate::new(coords.x, coords.y, coords.z - s2)),
                    (false, true, false) => (2, BlockCoordinate::new(coords.x - s2, coords.y, coords.z - s2)),
                    (false, true, true) => (3, BlockCoordinate::new(coords.x - s2, coords.y, coords.z)),
                    (true, false, true) => (4, BlockCoordinate::new(coords.x, coords.y - s2, coords.z)),
                    (true, false, false) => (5, BlockCoordinate::new(coords.x, coords.y - s2, coords.z - s2)),
                    (false, false, false) => (6, BlockCoordinate::new(coords.x - s2, coords.y - s2, coords.z - s2)),
                    (false, false, true) => (7, BlockCoordinate::new(coords.x - s2, coords.y - s2, coords.z)),
                };

                children[idx].block_id_at_and_scale(coords, scale / 2)
            }
        }
    }

    /// Returns the block at these coords in this LOD representation.
    pub fn mark_dirty(&mut self, coords: BlockCoordinate, root_scale: CoordinateType) {
        let scale = root_scale;
        match self {
            Lod::None => {}
            Lod::Single(_, dirty) => {
                *dirty = true;
            }
            Lod::Children(children) => {
                let s2 = (scale * CHUNK_DIMENSIONS) / 2;

                let (idx, coords) = match (coords.x < s2, coords.y < s2, coords.z < s2) {
                    (true, true, true) => (0, coords),
                    (true, true, false) => (1, BlockCoordinate::new(coords.x, coords.y, coords.z - s2)),
                    (false, true, false) => (2, BlockCoordinate::new(coords.x - s2, coords.y, coords.z - s2)),
                    (false, true, true) => (3, BlockCoordinate::new(coords.x - s2, coords.y, coords.z)),
                    (true, false, true) => (4, BlockCoordinate::new(coords.x, coords.y - s2, coords.z)),
                    (true, false, false) => (5, BlockCoordinate::new(coords.x, coords.y - s2, coords.z - s2)),
                    (false, false, false) => (6, BlockCoordinate::new(coords.x - s2, coords.y - s2, coords.z - s2)),
                    (false, false, true) => (7, BlockCoordinate::new(coords.x - s2, coords.y - s2, coords.z)),
                };

                children[idx].mark_dirty(coords, scale / 2)
            }
        }
    }
}
