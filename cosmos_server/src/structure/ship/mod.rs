//! Contains server-related ship logic

use bevy::prelude::App;

pub mod build_mode;
mod change_pilot_event_listener;
pub mod loading;
mod persistence;
pub mod server_ship_builder;
mod sync;

pub(super) fn register(app: &mut App) {
    change_pilot_event_listener::register(app);
    loading::register(app);
    persistence::register(app);
    sync::register(app);
    build_mode::register(app);
}
