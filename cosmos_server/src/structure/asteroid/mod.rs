//! Contains all the server logic for asteroids

use bevy::prelude::App;

mod dynamic;
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
    dynamic::register(app);
}
