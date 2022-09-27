use bevy::prelude::App;

pub mod ship_core;

pub fn register(app: &mut App) {
    ship_core::register(app);
}
