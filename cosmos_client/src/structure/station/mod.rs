//! Space station

use bevy::app::App;

pub mod client_station_builder;
pub mod create_station;

pub(super) fn register(app: &mut App) {
    client_station_builder::register(app);
    create_station::register(app);
}
