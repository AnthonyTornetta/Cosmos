//! Contains all the server logic for asteroids

use bevy::prelude::App;

pub mod generator;
pub mod generators;
mod persistence;
pub mod server_asteroid_builder;
mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
    generator::register(app);
    persistence::register(app);
    generators::register(app);
}
