//! Contains all the server logic for asteroids

use bevy::prelude::App;

mod generator;
pub mod server_asteroid_builder;
mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
    generator::register(app);
}
