//! Displays the information a player sees while piloting a ship

use bevy::app::App;

pub mod indicators;
mod stats_display;

pub(super) fn register(app: &mut App) {
    indicators::register(app);
    stats_display::register(app);
}
