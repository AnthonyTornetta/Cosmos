//! Prevents conflicts between different systems trying to disalbe a rigid body.
//!
//! TODO: add docs on how to use

use bevy::app::Update;
use bevy::prelude::App;
use bevy::prelude::Changed;
use bevy::prelude::Commands;
use bevy::prelude::Component;
use bevy::prelude::Entity;
use bevy::prelude::IntoSystemConfigs;
use bevy::prelude::Query;
use bevy::prelude::RemovedComponents;
use bevy::prelude::SystemSet;
use bevy::reflect::Reflect;
use bevy_rapier3d::prelude::RigidBodyDisabled;

use crate::netty::system_sets::NetworkingSystemsSet;

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
    mut removed_disable_rb: RemovedComponents<DisableRigidBody>,
    q_with_disable: Query<(Entity, &DisableRigidBody), Changed<DisableRigidBody>>,
) {
    for ent in removed_disable_rb.read() {
        if let Some(mut ecmds) = commands.get_entity(ent) {
            ecmds.remove::<RigidBodyDisabled>();
        }
    }

    for (ent, disable_rb) in q_with_disable.iter() {
        if disable_rb.should_be_disabled() {
            commands.entity(ent).insert(RigidBodyDisabled);
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
    app.configure_sets(Update, DisableRigidBodySet::DisableRigidBodies);

    app.add_systems(
        Update,
        disable_rigid_bodies
            .in_set(NetworkingSystemsSet::Between)
            .in_set(DisableRigidBodySet::DisableRigidBodies),
    )
    .register_type::<DisableRigidBody>();
}
