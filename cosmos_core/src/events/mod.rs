use bevy::prelude::{App, States};

pub mod block_events;
pub mod structure;
pub mod wrappers;

pub fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    block_events::register(app);
    structure::register(app, playing_state);
}
