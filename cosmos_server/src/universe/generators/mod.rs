//! These populate the universe with items. These do NOT spawn them as entities, only add them to
//! the [`UniverseSystem`].

#[cfg(doc)]
use super::UniverseSystem;

use bevy::prelude::*;

mod factions_generator;
pub mod generation;
mod pirate_station;
pub mod planet_spawner;
pub mod star;

pub(super) fn register(app: &mut App) {
    factions_generator::register(app);
    planet_spawner::register(app);
    star::register(app);
    pirate_station::register(app);
    generation::register(app);
}
