//! This stores LODs and the players they correspond to

use bevy::prelude::{Component, Entity};
use cosmos_core::structure::lod::{Lod, ReadOnlyLod};

#[derive(Debug, Component)]
/// Stores LODs and the players they correspond to
///
/// The PlayerLod's parent should always be the structure it is an lod of
pub struct PlayerLod {
    pub lod: Lod,
    pub read_only_lod: ReadOnlyLod,
    /// Will only ever contain serialized versions of LodNetworkMessage::SetLod. These are pre-computed to save time on the main thread
    pub deltas: Vec<Vec<u8>>,
    pub player: Entity,
}
