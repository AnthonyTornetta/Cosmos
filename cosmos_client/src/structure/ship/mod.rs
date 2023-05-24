//! Handles client-related ship things

use bevy::prelude::App;

pub mod client_ship_builder;

pub(super) fn register(app: &mut App) {
    client_ship_builder::register(app);
}
