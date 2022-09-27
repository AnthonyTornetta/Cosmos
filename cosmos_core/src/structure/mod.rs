use bevy::prelude::App;

pub mod chunk;
pub mod events;
pub mod planet;
pub mod ship;
pub mod structure;
pub mod structure_builder;

pub fn register(app: &mut App) {
    ship::register(app);
}
