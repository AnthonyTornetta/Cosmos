//! Pirate AI

use bevy::prelude::*;

pub mod ship_ai;
pub mod station;

pub(super) fn register(app: &mut App) {
    ship_ai::register(app);
    station::register(app);
}
