//! This stores LODs and the players they correspond to

use bevy::prelude::{Component, Entity};
use cosmos_core::structure::lod::{Lod, LodDelta};

#[derive(Debug, Component)]
/// Stores LODs and the players they correspond to
///
/// The PlayerLod's parent should always be the structure it is an lod of
pub struct PlayerLod {
    pub lod: Lod,
    pub deltas: Vec<LodDelta>,
    pub player: Entity,
}
