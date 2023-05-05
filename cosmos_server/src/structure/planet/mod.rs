//! Contains server-side logic for the planet

use bevy::prelude::*;

pub mod biosphere;
mod chunk;
pub mod generation;
mod persistence;
pub mod server_planet_builder;
mod sync;

pub(super) fn register(app: &mut App) {
    biosphere::register(app);
    persistence::register(app);
    sync::register(app);
    generation::register(app);
    chunk::register(app);
}
