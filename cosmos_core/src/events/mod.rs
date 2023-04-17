//! This whole module should be removed - it's the definition of low cohesion.
//!
//! Events should be present in the module they are for, not some random events module.

use bevy::prelude::{App, States};

pub mod block_events;
pub mod structure;
pub mod wrappers;

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    block_events::register(app);
    structure::register(app, playing_state);
}
