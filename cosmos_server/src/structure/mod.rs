use bevy::prelude::App;

pub mod planet;
pub mod server_structure_builder;
pub mod ship;
pub mod systems;

pub(crate) fn register(app: &mut App) {
    ship::register(app);
    systems::register(app);
}
