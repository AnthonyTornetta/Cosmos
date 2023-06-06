//! Contains information need to have physics operate successfully

use bevy::prelude::App;

pub mod gravity_system;
pub mod location;
pub mod player_world;
mod stop_near_unloaded_chunks;
pub mod structure_physics;

pub(super) fn register(app: &mut App) {
    structure_physics::register(app);
    gravity_system::register(app);
    location::register(app);
    player_world::register(app);
    stop_near_unloaded_chunks::register(app);
}
