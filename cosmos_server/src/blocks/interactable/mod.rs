use bevy::prelude::App;

pub mod block_interact_event;
pub mod ship_core;

pub fn register(app: &mut App) {
    block_interact_event::register(app);
    ship_core::register(app);
}
