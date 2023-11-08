//! Contains all structure-related information for the server

use bevy::prelude::App;

pub mod asteroid;
pub mod block_health;
pub mod planet;
pub mod server_structure_builder;
pub mod ship;
pub mod systems;

pub(super) fn register(app: &mut App) {
    ship::register(app);
    systems::register(app);
    planet::register(app);
    block_health::register(app);
    asteroid::register(app);
}
