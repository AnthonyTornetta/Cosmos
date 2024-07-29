use bevy::prelude::{App, States};
use pilot_change_event_listener::PilotEventSystemSet;

mod pilot_change_event_listener;

pub type ShipEventListenerSet = PilotEventSystemSet;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    pilot_change_event_listener::register(app, playing_state);
}
