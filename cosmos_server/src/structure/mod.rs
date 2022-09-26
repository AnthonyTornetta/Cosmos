use bevy::prelude::App;

pub mod planet;
pub mod server_structure_builder;
pub mod ship;

pub fn register(app: &mut App) {
    ship::register(app);
}
