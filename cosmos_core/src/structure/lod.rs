use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use super::lod_chunk::LodChunk;

#[derive(Serialize, Deserialize, Component, Debug)]
/// Represents a reduced-detail version of a planet
pub enum Lod {
    /// No Lod here - this means there should be an actual chunk here
    None,
    /// Represents a single chunk of blocks at any scale.
    Single(Box<LodChunk>),
    /// Breaks a single cube into 8 sub-cubes.
    ///
    /// The indicies of each cube follow a clockwise direction starting on the bottom-left-back
    ///
    /// ```
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
    Children(Box<[Lod; 8]>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetLodMessage {
    pub structure: Entity,
    pub serialized_lod: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LodNetworkMessage {
    SetLod(SetLodMessage),
}
