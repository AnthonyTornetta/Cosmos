//! This stores LODs and the players they correspond to

use bevy::prelude::{Component, Entity};
use cosmos_core::structure::lod::Lod;

#[derive(Debug, Component)]
/// Stores LODs and the players they correspond to
pub struct PlayerLod {
    pub lod: Lod,
    pub player: Entity,
}
