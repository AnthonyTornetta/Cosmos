//! Contains server-side logic for the universe & how it's generated

use bevy::prelude::App;

pub mod asteroid_spawner;
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
    asteroid_spawner::register(app);
    spawners::register(app);
}
