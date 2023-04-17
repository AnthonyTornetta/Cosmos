//! Handles all the player's cameras

use bevy::prelude::App;

pub mod camera_controller;

pub(super) fn register(app: &mut App) {
    camera_controller::register(app);
}
