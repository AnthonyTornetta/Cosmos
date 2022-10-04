use bevy::{ecs::schedule::StateData, prelude::App};

pub mod pilot_change_event_listener;

pub fn register<T: StateData + Clone>(app: &mut App, playing_state: T) {
    pilot_change_event_listener::register(app, playing_state);
}
