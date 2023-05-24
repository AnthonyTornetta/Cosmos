//! Handles all client asteroid logic

use bevy::prelude::App;

pub mod client_asteroid_builder;
pub mod sync;

pub(super) fn register(app: &mut App) {
    client_asteroid_builder::register(app);
    sync::register(app);
}
