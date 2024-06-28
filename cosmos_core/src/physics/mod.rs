//! Contains information need to have physics operate successfully

use bevy::{
    app::Startup,
    ecs::system::ResMut,
    math::Vec3,
    prelude::{App, States},
};
use bevy_rapier3d::plugin::{RapierContext, DEFAULT_WORLD_ID};

pub mod block_colliders;
pub mod collision_handling;
pub mod gravity_system;
pub mod location;
pub mod player_world;
mod stop_near_unloaded_chunks;
pub mod structure_physics;

fn remove_gravity(mut ctx: ResMut<RapierContext>) {
    ctx.get_world_mut(DEFAULT_WORLD_ID)
        .expect("This should exist at startup")
        .set_gravity(Vec3::ZERO);
}

pub(super) fn register<T: States + Copy>(app: &mut App, post_loading_state: T) {
    app.add_systems(Startup, remove_gravity);

    structure_physics::register(app);
    gravity_system::register(app);
    location::register(app);
    player_world::register(app);
    collision_handling::register(app);
    stop_near_unloaded_chunks::register(app);
    block_colliders::register(app, post_loading_state);
}
