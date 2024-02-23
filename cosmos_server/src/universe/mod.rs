//! Contains server-side logic for the universe & how it's generated

use bevy::prelude::App;

pub mod asteroid_spawner;
pub mod generation;
pub mod planet_spawner;
pub mod spawners;
pub mod star;

pub(super) fn register(app: &mut App) {
    star::register(app);
    generation::register(app);
    planet_spawner::register(app);
    asteroid_spawner::register(app);
    spawners::register(app);
}
