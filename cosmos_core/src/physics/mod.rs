//! Contains information need to have physics operate successfully

use bevy::prelude::{App, States};

pub mod block_colliders;
pub mod gravity_system;
pub mod location;
pub mod player_world;
mod stop_near_unloaded_chunks;
pub mod structure_physics;

pub(super) fn register<T: States + Copy>(app: &mut App, post_loading_state: T) {
    structure_physics::register(app);
    gravity_system::register(app);
    location::register(app);
    player_world::register(app);
    stop_near_unloaded_chunks::register(app);
    block_colliders::register(app, post_loading_state);
}
