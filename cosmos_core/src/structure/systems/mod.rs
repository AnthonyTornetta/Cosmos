use bevy::{ecs::schedule::StateData, prelude::App};

pub mod energy_storage_system;

pub fn register<T: StateData + Clone>(app: &mut App, loading_state: T) {
    energy_storage_system::register(app, loading_state);
}
