use bevy::{ecs::schedule::StateData, prelude::*};

pub mod energy_generation_system;
pub mod energy_storage_system;
pub mod thruster_system;

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_state: T) {
    energy_storage_system::register(app, post_loading_state.clone(), playing_state.clone());
    energy_generation_system::register(app, post_loading_state.clone(), playing_state.clone());
    thruster_system::register(app, post_loading_state, playing_state);
}
