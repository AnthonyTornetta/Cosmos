//! Contains information & functionality for various types of entities
//!
//! This is far to generic of a module, and should be removed at some point in favor of more specific modules.

use bevy::prelude::App;

pub mod player;

pub(super) fn register(app: &mut App) {
    player::register(app);
}
