//! Ways of displaying reduced-detail versions of dynamic structures

use std::{
    backtrace::Backtrace,
    sync::{Arc, Mutex},
};

use bevy::{
    log::warn,
    prelude::{Component, Entity},
};
use serde::{Deserialize, Serialize};

use crate::block::blocks::AIR_BLOCK_ID;

use super::{
    block_storage::BlockStorer,
    chunk::CHUNK_DIMENSIONS,
    coordinates::CoordinateType,
    lod_chunk::LodChunk,
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
        self.block_id_at(coords, root_scale) != AIR_BLOCK_ID
    }

    /// Returns the block at these coords in this LOD representation.
    pub fn block_id_at(&self, coords: BlockCoordinate, root_scale: CoordinateType) -> u16 {
        let scale = root_scale;
        match self {
            Lod::None => AIR_BLOCK_ID,
            Lod::Single(lod, _) => {
                // let scale = scale / 2;
                let c = BlockCoordinate::new(coords.x / scale, coords.y / scale, coords.z / scale);

                if let Ok(chunk_block_coord) = ChunkBlockCoordinate::try_from(c) {
                    lod.block_at(chunk_block_coord)
                } else {
                    AIR_BLOCK_ID
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

                children[idx].block_id_at(coords, scale / 2)
            }
        }
    }
}

impl LodDelta {
    /// Creates an lod based off this delta.
    ///
    /// # Panics
    /// if self contains any LodDelta::NoChange because these must have a corresponding lod.
    pub fn create_lod(self) -> Lod {
        match self {
            LodDelta::Children(children) => {
                let [c0, c1, c2, c3, c4, c5, c6, c7] = *children;

                Lod::Children(Box::new([
                    c0.create_lod(),
                    c1.create_lod(),
                    c2.create_lod(),
                    c3.create_lod(),
                    c4.create_lod(),
                    c5.create_lod(),
                    c6.create_lod(),
                    c7.create_lod(),
                ]))
            }
            LodDelta::None => Lod::None,
            LodDelta::Single(chunk) => Lod::Single(chunk, true),
            LodDelta::NoChange => {
                // Forcibly capture the backtrace regardless of environment variable configuration
                warn!("Got no change but there wasn't an lod entry for that no change!");
                warn!("Backtrace: \n{}", Backtrace::force_capture());

                // panic!("Cannot have no change with no lod given!");
                Lod::None
            }
        }
    }

    /// Applies the delta changes to a present lod
    ///
    /// # Panics
    /// if self contains any LodDelta::NoChange and there is no matching lod for that.
    pub fn apply_changes(self, lod: &mut Lod) {
        match self {
            LodDelta::Children(children) => {
                let [c0, c1, c2, c3, c4, c5, c6, c7] = *children;

                match lod {
                    Lod::Children(lod_children) => {
                        c0.apply_changes(&mut lod_children[0]);
                        c1.apply_changes(&mut lod_children[1]);
                        c2.apply_changes(&mut lod_children[2]);
                        c3.apply_changes(&mut lod_children[3]);
                        c4.apply_changes(&mut lod_children[4]);
                        c5.apply_changes(&mut lod_children[5]);
                        c6.apply_changes(&mut lod_children[6]);
                        c7.apply_changes(&mut lod_children[7]);
                    }
                    _ => {
                        *lod = Lod::Children(Box::new([
                            c0.create_lod(),
                            c1.create_lod(),
                            c2.create_lod(),
                            c3.create_lod(),
                            c4.create_lod(),
                            c5.create_lod(),
                            c6.create_lod(),
                            c7.create_lod(),
                        ]));
                    }
                }
            }
            LodDelta::None => {
                *lod = Lod::None;
            }
            LodDelta::Single(chunk) => {
                *lod = Lod::Single(chunk, true);
            }
            LodDelta::NoChange => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// Sends an Lod to the client
pub struct SetLodMessage {
    /// The structure this lod belongs to
    pub structure: Entity,
    /// The LodDelta serialized
    pub serialized_lod: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
/// All the lod network messages
pub enum LodNetworkMessage {
    /// Set the lod to this lod
    SetLod(SetLodMessage),
}
