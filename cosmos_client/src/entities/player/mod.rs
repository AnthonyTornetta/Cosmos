//! Contains systems and components for the player

use bevy::prelude::App;

pub mod player_movement;
pub mod render_distance;

pub(super) fn register(app: &mut App) {
    render_distance::register(app);
    player_movement::register(app);
}
