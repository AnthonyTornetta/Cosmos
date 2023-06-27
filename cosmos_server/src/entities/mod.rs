//! Contains all server information about various entities

use bevy::prelude::App;

pub mod player;

pub(super) fn register(app: &mut App) {
    player::register(app);
}
