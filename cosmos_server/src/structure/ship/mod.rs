use bevy::prelude::App;

pub mod change_pilot_event_listener;
pub mod server_ship_builder;

pub fn register(app: &mut App) {
    change_pilot_event_listener::register(app);
}
