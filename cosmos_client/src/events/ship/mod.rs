//! Represents the different types of ship events

use bevy::prelude::App;

pub mod create_ship;
pub mod set_ship_event;

pub(super) fn register(app: &mut App) {
    create_ship::register(app);
    set_ship_event::register(app);
}
