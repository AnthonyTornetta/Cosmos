use bevy::prelude::{App, States};

pub mod pilot_change_event_listener;

pub fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot_change_event_listener::register(app, playing_state);
}
