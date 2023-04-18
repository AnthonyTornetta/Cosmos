//! Contains server-related information about blocks

use bevy::prelude::App;

pub mod block_events;

pub(super) fn register(app: &mut App) {
    block_events::register(app);
}
