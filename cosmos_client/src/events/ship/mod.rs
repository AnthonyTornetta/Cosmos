use bevy::prelude::App;

pub mod create_ship;

pub fn register(app: &mut App) {
    create_ship::register(app);
}
