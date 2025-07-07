//! Handles all the interactable blocks

use bevy::prelude::App;

mod door;
mod dye_machine;
mod gravity_well;
mod ship_core;
pub mod storage;

pub(super) fn register(app: &mut App) {
    dye_machine::register(app);
    ship_core::register(app);
    storage::register(app);
    gravity_well::register(app);
    door::register(app);
}
