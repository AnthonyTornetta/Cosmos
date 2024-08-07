//! Events that are relevent to structures

use bevy::prelude::{App, States};
use ship::ShipEventListenerSet;

pub mod change_pilot_event;
mod ship;

/// Systems that listen to structure events are in here (currently just ships use this)
pub type StructureEventListenerSet = ShipEventListenerSet;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    change_pilot_event::register(app);
    ship::register(app, playing_state);
}
