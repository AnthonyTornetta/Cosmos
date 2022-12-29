use bevy::prelude::App;

pub mod gravity_system;
pub mod structure_physics;

pub fn register(app: &mut App) {
    structure_physics::register(app);
    gravity_system::register(app);
}
