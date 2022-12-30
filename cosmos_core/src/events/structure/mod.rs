use bevy::{ecs::schedule::StateData, prelude::App};

pub mod change_pilot_event;
pub mod ship;

pub fn register<T: StateData + Clone + Copy>(app: &mut App, playing_state: T) {
    change_pilot_event::register(app);
    ship::register(app, playing_state);
}
