//! Handles fluid-specific block logic, such as their colliders.

use bevy::prelude::*;
use bevy_rapier3d::geometry::Group;

use crate::{
    block::Block,
    physics::block_colliders::*,
    registry::{Registry, identifiable::Identifiable},
};

/// Collision group all fluid blocks will use for their colliders
pub const FLUID_COLLISION_GROUP: Group = Group::GROUP_4;

fn create_fluid_colliders(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockCollider>>) {
    for block in blocks.iter().filter(|b| b.is_fluid()) {
        if !registry.contains(block.unlocalized_name()) {
            registry.register(BlockCollider::new(BlockColliderType::Fluid, block.unlocalized_name()));
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    app.add_systems(
        OnEnter(post_loading_state),
        create_fluid_colliders.in_set(ColliderRegistrationSet::PreRegisterRemainingColliders),
    );
}
