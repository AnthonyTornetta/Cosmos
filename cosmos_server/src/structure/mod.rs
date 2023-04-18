//! Contains all structure-related information for the server

use bevy::prelude::App;

pub mod block_health;
pub mod persistence;
pub mod planet;
pub mod saving;
pub mod server_structure_builder;
pub mod ship;
pub mod systems;

pub(super) fn register(app: &mut App) {
    ship::register(app);
    systems::register(app);
    planet::register(app);
    block_health::register(app);
    saving::register(app);
    persistence::register(app);
}
