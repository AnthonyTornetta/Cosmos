//! Handles all server block stuff

use bevy::prelude::App;

mod block_events;
pub mod interactable;
pub mod multiblock;

pub(super) fn register(app: &mut App) {
    interactable::register(app);
    block_events::register(app);
    multiblock::register(app);
}
