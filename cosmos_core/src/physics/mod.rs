use bevy::prelude::App;

pub mod structure_physics;
pub mod gravity_system;

pub fn register(app: &mut App) {
    structure_physics::register(app);
    gravity_system::register(app);
}
