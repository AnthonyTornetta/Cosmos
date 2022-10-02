use bevy::{ecs::schedule::StateData, prelude::App};

pub mod energy_generation_system;
pub mod energy_storage_system;

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_game_state: T) {
    energy_storage_system::register(app, post_loading_state.clone(), playing_game_state.clone());
    energy_generation_system::register(app, post_loading_state.clone(), playing_game_state.clone());
}
