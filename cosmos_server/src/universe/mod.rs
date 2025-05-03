//! Contains server-side logic for the universe & how it's generated

use bevy::prelude::App;

mod factions_generator;
pub mod galaxy_generation;
pub mod generation;
pub mod map;
pub mod planet_spawner;
pub mod spawners;
pub mod star;

pub(super) fn register(app: &mut App) {
    galaxy_generation::register(app);
    map::register(app);
    star::register(app);
    generation::register(app);
    planet_spawner::register(app);
    spawners::register(app);
    factions_generator::register(app);
}
