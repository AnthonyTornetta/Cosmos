use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub enum FactionRelation {
    Ally,
    #[default]
    Neutral,
    Enemy,
}

pub struct Faction {
    id: u64,
    name: String,
    players: Vec<EntityId>,
}

pub(super) fn register(app: &mut App) {}
