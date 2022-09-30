use bevy::{ecs::schedule::StateData, prelude::App};

pub mod chunk;
pub mod events;
pub mod planet;
pub mod ship;
pub mod structure;
pub mod structure_builder;
pub mod systems;

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_game_state: T) {
    systems::register(app, post_loading_state, playing_game_state);
    ship::register(app);
}
