//! Contains all structure-related information for the server

use bevy::{log::info, prelude::App};

pub mod asteroid;
pub mod block_health;
pub mod persistence;
pub mod planet;
pub mod server_structure_builder;
pub mod ship;
pub mod systems;

pub(super) fn register(app: &mut App) {
    info!(".ship");
    ship::register(app);
    info!(".systems");
    systems::register(app);
    info!(".planet");
    planet::register(app);
    info!(".block_health");
    block_health::register(app);
    info!(".asteroid");
    asteroid::register(app);

    persistence::register(app);
}
