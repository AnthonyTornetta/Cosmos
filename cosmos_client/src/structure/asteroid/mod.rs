//! Handles all client asteroid logic

use bevy::prelude::App;

pub mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
}
