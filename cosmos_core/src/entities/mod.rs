//! Contains information & functionality for various types of entities
//!
//! This is far to generic of a module, and should be removed at some point in favor of more specific modules.

use std::fmt::Display;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod health;
pub mod player;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Hash)]
/// NOT ALL ENTITIES WILL HAVE THIS ON THEM!
///
/// Only entities that have been loaded or saved will have this. This is a unique identifier for
/// this entity.
pub struct EntityId(Uuid);

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from(self.0))
    }
}

impl EntityId {
    /// Creates a new EntityID.
    ///
    /// * `id` This should be unique to only this entity. If this isn't unique, the entity may not be loaded/saved correctly
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Creates a new EntityId
    pub fn generate() -> Self {
        Self::new(Uuid::new_v4())
    }
}

pub(super) fn register(app: &mut App) {
    health::register(app);
    player::register(app);
}
