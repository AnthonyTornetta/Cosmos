//! Represents the cosmos input systems

use bevy::prelude::App;

pub mod inputs;

pub(super) fn register(app: &mut App) {
    inputs::register(app);
}
