use bevy::{ecs::schedule::StateData, prelude::App};

pub mod thruster_system;

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_state: T) {
    thruster_system::register(app, post_loading_state, playing_state);
}
