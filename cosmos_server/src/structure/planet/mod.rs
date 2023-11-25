//! Contains server-side logic for the planet

use bevy::prelude::*;

pub mod biosphere;
mod chunk;
pub mod generation;
mod lods;
mod persistence;
pub mod server_planet_builder;
mod sync;

pub(super) fn register(app: &mut App) {
    info!("..biosphere");
    biosphere::register(app);
    info!("..persistence");
    persistence::register(app);
    info!("..sync");
    sync::register(app);
    info!("..generation");
    generation::register(app);
    info!("..chunk");
    chunk::register(app);
    info!("..lods");
    lods::register(app);
}
