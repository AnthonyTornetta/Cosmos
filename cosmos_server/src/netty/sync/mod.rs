//! Handles the syncing of various items

use bevy::prelude::App;

pub mod sync_bodies;

pub(super) fn register(app: &mut App) {
    sync_bodies::register(app);
}
