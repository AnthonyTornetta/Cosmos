//! Space station

use bevy::app::App;

pub mod create_station;

pub(super) fn register(app: &mut App) {
    create_station::register(app);
}
