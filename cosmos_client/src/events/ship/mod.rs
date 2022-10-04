use bevy::prelude::App;

pub mod create_ship;
pub mod set_ship_event;

pub fn register(app: &mut App) {
    create_ship::register(app);
    set_ship_event::register(app);
}
