use bevy::{ecs::schedule::StateData, prelude::App};

pub mod chunk;
pub mod events;
pub mod planet;
pub mod ship;
pub mod structure;
pub mod structure_builder;
pub mod systems;

pub fn register<T: StateData + Clone>(app: &mut App, loading_state: T) {
    systems::register(app, loading_state);
    ship::register(app);
}
