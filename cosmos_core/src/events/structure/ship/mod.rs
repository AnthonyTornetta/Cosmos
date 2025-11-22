use bevy::prelude::{App, States};
use pilot_change_event_listener::PilotMessageSystemSet;

mod pilot_change_event_listener;

pub type ShipMessageListenerSet = PilotMessageSystemSet;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot_change_event_listener::register(app, playing_state);
}
