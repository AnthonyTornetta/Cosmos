//! Handles planet generation

use bevy::prelude::App;

pub mod planet_generator;

pub(super) fn register(app: &mut App) {
    planet_generator::register(app);
}
