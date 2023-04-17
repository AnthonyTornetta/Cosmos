//! Handles all the interactable blocks

use bevy::prelude::App;

mod ship_core;

pub(super) fn register(app: &mut App) {
    ship_core::register(app);
}
