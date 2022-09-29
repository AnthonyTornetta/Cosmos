use bevy::{ecs::schedule::StateData, prelude::App};

pub mod block;
pub mod block_builder;
pub mod blocks;

pub fn register<T: StateData + Clone>(app: &mut App, pre_loading_state: T, loading_state: T) {
    blocks::register(app, pre_loading_state, loading_state);
}
