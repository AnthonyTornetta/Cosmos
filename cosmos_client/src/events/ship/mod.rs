use bevy::prelude::App;

pub mod create_ship;
pub mod pilot_change_event_listener;

pub fn register(app: &mut App) {
    create_ship::register(app);
    pilot_change_event_listener::register(app);
}
