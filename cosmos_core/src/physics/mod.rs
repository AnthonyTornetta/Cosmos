//! Contains information need to have physics operate successfully

use bevy::prelude::*;
use bevy_rapier3d::prelude::RigidBody;

use crate::ecs::sets::FixedUpdateSet;
pub mod block_colliders;
pub mod collision_handling;
pub mod disable_rigid_body;
pub mod gravity_system;
pub mod location;
pub mod player_world;
mod stop_near_unloaded_chunks;
pub mod structure_physics;

#[derive(Component)]
struct PrevRb(RigidBody);

fn fix_rapier_bug(mut commands: Commands, mut q_rb: Query<(Entity, &mut RigidBody, Option<&PrevRb>), Added<RigidBody>>) {
    for (ent, mut rb, last_rb) in q_rb.iter_mut() {
        if *rb == RigidBody::Fixed {
            // if last_rb.map(|x| x.0 != *rb).unwrap_or(true) {
            commands.entity(ent).insert(PrevRb(*rb));
            rb.set_changed();
            // }
        }
    }
}

pub(super) fn register<T: States + Copy>(app: &mut App, post_loading_state: T) {
    structure_physics::register(app);
    gravity_system::register(app);
    location::register(app);
    player_world::register(app);
    collision_handling::register(app);
    stop_near_unloaded_chunks::register(app);
    block_colliders::register(app, post_loading_state);
    disable_rigid_body::register(app);

    app.add_systems(FixedUpdate, fix_rapier_bug.in_set(FixedUpdateSet::PrePhysics));
}
