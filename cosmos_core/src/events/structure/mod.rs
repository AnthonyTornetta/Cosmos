//! Messages that are relevent to structures

use bevy::prelude::{App, States};
use ship::ShipMessageListenerSet;

pub mod change_pilot_event;
mod ship;
pub mod structure_event;

/// Systems that listen to structure events are in here (currently just ships use this)
pub type StructureMessageListenerSet = ShipMessageListenerSet;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    change_pilot_event::register(app);
    ship::register(app, playing_state);
}
