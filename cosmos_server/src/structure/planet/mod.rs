//! Contains server-side logic for the planet

use bevy::prelude::*;

pub mod biosphere;
pub mod generation;
mod persistence;
pub mod server_planet_builder;
mod sync;

pub(crate) fn register(app: &mut App) {
    biosphere::register(app);
    persistence::register(app);
    sync::register(app);
}
