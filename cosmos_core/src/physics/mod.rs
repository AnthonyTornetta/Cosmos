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
struct DidFix;

/// honestly I just can't care enough to fix this properly. I'm so tired of physics issues
///
/// For some reason, fixed rigid bodies just don't work when added. But they work if you swap them
/// to anything else then back to fixed. This is so evil.
fn fix_rapier_bug(mut commands: Commands, mut q_rb: Query<(Entity, &mut RigidBody), Without<DidFix>>) {
    for (ent, mut rb) in q_rb.iter_mut() {
        if *rb == RigidBody::Fixed {
            *rb = RigidBody::KinematicPositionBased;
            commands.entity(ent).insert(DidFix);
        }
    }
}

fn revert_rb(q_entity: Query<Entity, Added<DidFix>>, mut commands: Commands) {
    for e in q_entity.iter() {
        commands.entity(e).insert(RigidBody::Fixed);
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

    app.add_systems(FixedUpdate, (revert_rb, fix_rapier_bug).chain().in_set(FixedUpdateSet::PrePhysics));
}
