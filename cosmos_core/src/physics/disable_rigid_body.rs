//! Prevents conflicts between different systems trying to disalbe a rigid body.
//!
//! TODO: add docs on how to use

use bevy::prelude::*;
use bevy_rapier3d::prelude::RigidBodyDisabled;

use crate::{
    ecs::sets::FixedUpdateSet,
    utils::ecs::{FixedUpdateRemovedComponents, register_fixed_update_removed_component},
};

#[derive(Component, Default, Reflect, Debug)]
/// Instead of directly using [`RigidBodyDisabled`], use this to not risk overwriting other systems
///
/// Simply use [`Self::add_reason`] and [`Self::remove_reason`] to add your reason for disabling
/// this rigidbody. If there are no reasons, the body will be enabled. If there are any current
/// reasons, the body will be disabled.
pub struct DisableRigidBody {
    reasons: Vec<String>,
}

impl DisableRigidBody {
    /// [`Self::default`] + [`Self::add_reason`]
    pub fn new_with_reason(reason: &'static str) -> Self {
        Self {
            reasons: vec![reason.into()],
        }
    }

    /// Returns true if there are any reasons present
    pub fn should_be_disabled(&self) -> bool {
        !self.reasons.is_empty()
    }

    /// Adds your reason for this being disabled. Make sure to make the reason unique.
    pub fn add_reason(&mut self, reason: &'static str) {
        if self.contains_reason(reason) {
            return;
        }
        self.reasons.push(reason.into());
    }

    /// Removes your reason for this being disabled.
    pub fn remove_reason(&mut self, reason: &'static str) {
        if let Some((idx, _)) = self.reasons.iter().enumerate().find(|(_, x)| *x == reason) {
            self.reasons.remove(idx);
        }
    }

    /// Checks if this contains the given reason
    pub fn contains_reason(&self, reason: &str) -> bool {
        self.reasons.iter().any(|x| x.as_str() == reason)
    }
}

fn disable_rigid_bodies(
    mut commands: Commands,
    removed_disable_rb: FixedUpdateRemovedComponents<DisableRigidBody>,
    q_with_disable: Query<(Entity, &DisableRigidBody), Changed<DisableRigidBody>>,
) {
    for ent in removed_disable_rb.read() {
        if let Ok(mut ecmds) = commands.get_entity(ent) {
            ecmds.remove::<RigidBodyDisabled>();
        }
    }

    for (ent, disable_rb) in q_with_disable.iter() {
        if disable_rb.should_be_disabled() {
            // TODO: rapier is so bugged I can't even begin to describe my frustration
            // This just crashes the physics sim half the time. I genuinely have no idea how to fix
            // this. Maybe add a lock in place component?
            // commands.entity(ent).insert(RigidBodyDisabled);
        } else {
            commands.entity(ent).remove::<RigidBodyDisabled>();
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Responsible for adding or rmoving the [`RigidBodyDisabled`] to entities with the [`DisableRigidBody`]
/// component
pub enum DisableRigidBodySet {
    /// Responsible for adding or rmoving the [`RigidBodyDisabled`] to entities with the [`DisableRigidBody`]
    /// component
    DisableRigidBodies,
}

pub(super) fn register(app: &mut App) {
    register_fixed_update_removed_component::<DisableRigidBody>(app);

    app.configure_sets(FixedUpdate, DisableRigidBodySet::DisableRigidBodies);

    app.add_systems(
        FixedUpdate,
        disable_rigid_bodies
            .in_set(FixedUpdateSet::PostLocationSyncingPostPhysics)
            .in_set(DisableRigidBodySet::DisableRigidBodies),
    )
    .register_type::<DisableRigidBody>();
}
