use bevy::prelude::App;

pub mod loading;
pub mod planet;
pub mod server_structure_builder;
pub mod ship;
pub mod systems;

pub fn register(app: &mut App) {
    ship::register(app);
    loading::register(app);
}
