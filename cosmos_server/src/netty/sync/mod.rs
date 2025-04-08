//! Handles the syncing of various items

use bevy::prelude::App;

pub mod components;
pub mod registry;
pub mod sync_bodies;

pub(super) fn register(app: &mut App) {
    components::register(app);
    sync_bodies::register(app);
    registry::register(app);
}
