//! Handles all server block stuff

use bevy::prelude::App;

mod block_events;
pub mod data;
pub mod drops;
pub mod interactable;
pub mod multiblock;
mod updates;

pub(super) fn register(app: &mut App) {
    interactable::register(app);
    block_events::register(app);
    multiblock::register(app);
    updates::register(app);
    data::register(app);
    drops::register(app);
}
