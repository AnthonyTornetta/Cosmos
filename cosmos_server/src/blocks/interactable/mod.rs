//! Handles all the interactable blocks

use bevy::prelude::App;

mod gravity_well;
mod ship_core;
mod storage;

pub(super) fn register(app: &mut App) {
    ship_core::register(app);
    storage::register(app);
    gravity_well::register(app);
}
