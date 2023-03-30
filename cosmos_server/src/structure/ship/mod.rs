use bevy::prelude::App;

pub mod change_pilot_event_listener;
pub mod loading;
pub mod persistence;
pub mod server_ship_builder;
pub mod sync;

pub(super) fn register(app: &mut App) {
    change_pilot_event_listener::register(app);
    loading::register(app);
    persistence::register(app);
    sync::register(app);
}
