use bevy::prelude::App;

pub mod structure_physics;

pub fn register(app: &mut App) {
    structure_physics::register(app);
}
