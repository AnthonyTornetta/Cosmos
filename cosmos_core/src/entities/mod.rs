//! Contains information & functionality for various types of entities
//!
//! This is far to generic of a module, and should be removed at some point in favor of more specific modules.

use std::fmt::Display;

use bevy::prelude::*;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};

pub mod player;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
/// NOT ALL ENTITIES WILL HAVE THIS ON THEM!
///
/// Only entities that have been loaded or saved will have this. This is a unique identifier for
/// this entity.
pub struct EntityId(String);
// TODO: This should really be a uuid, not sure why I did this dumb custom string approach

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl EntityId {
    /// Creates a new EntityID.
    ///
    /// * `id` This should be unique to only this entity. If this isn't unique, the entity may not be loaded/saved correctly
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Creates a new EntityId
    pub fn generate() -> Self {
        Self::new(
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect::<String>(),
        )
    }

    /// Returns the entity id as a string
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

pub(super) fn register(app: &mut App) {
    player::register(app);
}
