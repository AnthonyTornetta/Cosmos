use bevy::prelude::App;

pub mod gravity_system;
pub mod location;
pub mod player_world;
mod structure_physics;

pub(super) fn register(app: &mut App) {
    structure_physics::register(app);
    gravity_system::register(app);
    location::register(app);
    player_world::register(app);
}
