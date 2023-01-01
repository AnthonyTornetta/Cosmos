use bevy::{ecs::schedule::StateData, prelude::App};

pub mod block_events;
pub mod structure;

pub fn register<T: StateData + Clone + Copy>(app: &mut App, playing_state: T) {
    block_events::register(app);
    structure::register(app, playing_state);
}
