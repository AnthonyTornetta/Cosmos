//! Contains information & functionality for various types of entities
//!
//! This is far to generic of a module, and should be removed at some point in favor of more specific modules.

use std::fmt::Display;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod player;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
/// NOT ALL ENTITIES WILL HAVE THIS ON THEM!
///
/// Only entities that have been loaded or saved will have this. This is a unique identifier for
/// this entity.
pub struct EntityId(String);

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub(super) fn register(app: &mut App) {
    player::register(app);
}
