use bevy::prelude::App;

pub mod block;
pub mod ship;

pub fn register(app: &mut App) {
    block::register(app);
    ship::register(app);
}
