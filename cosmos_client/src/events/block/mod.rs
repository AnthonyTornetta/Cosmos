//! Handles all block events

use bevy::prelude::App;

pub mod block_events;

pub(super) fn register(app: &mut App) {
    block_events::register(app);
}
