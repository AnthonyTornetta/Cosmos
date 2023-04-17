use bevy::prelude::{App, States};

mod pilot_change_event_listener;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot_change_event_listener::register(app, playing_state);
}
