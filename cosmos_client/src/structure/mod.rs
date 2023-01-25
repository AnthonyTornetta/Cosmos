use bevy::prelude::App;

pub mod chunk_retreiver;
pub mod client_structure_builder;
pub mod planet;
pub mod ship;
pub mod systems;

pub fn register(app: &mut App) {
    systems::register(app);
}
