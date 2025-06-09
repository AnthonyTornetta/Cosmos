//! Contains server-side logic for the planet

use bevy::prelude::*;

pub mod biosphere;
pub mod chunk;
pub mod generation;
pub mod persistence;
mod planet_rotation;
mod sync;

pub(super) fn register(app: &mut App) {
    planet_rotation::register(app);
    biosphere::register(app);
    persistence::register(app);
    sync::register(app);
    generation::register(app);
    chunk::register(app);
}
