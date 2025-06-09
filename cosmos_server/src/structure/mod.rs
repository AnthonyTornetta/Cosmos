//! Contains all structure-related information for the server

use bevy::prelude::App;

pub mod asteroid;
pub mod block_health;
pub mod persistence;
pub mod planet;
pub mod shared;
pub mod ship;
pub mod station;
pub mod systems;

pub(super) fn register(app: &mut App) {
    ship::register(app);
    systems::register(app);
    planet::register(app);
    block_health::register(app);
    asteroid::register(app);

    persistence::register(app);
    shared::register(app);
    station::register(app);
}
