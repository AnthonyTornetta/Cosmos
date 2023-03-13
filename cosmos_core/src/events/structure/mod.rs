use bevy::prelude::{App, States};

pub mod change_pilot_event;
pub mod ship;

pub fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    change_pilot_event::register(app);
    ship::register(app, playing_state);
}
