use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};

use super::lod_chunk::LodChunk;

#[derive(Serialize, Deserialize, Component, Debug)]
pub enum Lod {
    None,
    Single(Box<LodChunk>),
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
