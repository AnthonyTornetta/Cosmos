//! Handles the [`Shield`] component and its shared logic

use bevy::prelude::*;
use bevy_rapier3d::{
    geometry::{Collider, ColliderMassProperties, Group, Sensor},
    plugin::{PhysicsSet, RapierContextEntityLink},
    prelude::CollisionGroups,
};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{IdentifiableComponent, SyncableComponent, sync_component};

use super::{coordinates::BlockCoordinate, shared::DespawnWithStructure};

#[derive(Component, Reflect, Clone, Debug, Serialize, Deserialize, PartialEq)]
/// Blocks projectiles that are within the shields bounds
pub struct Shield {
    /// How big the shield's radius is
    pub radius: f32,
    /// Where the shield is
    pub block_coord: BlockCoordinate,
    /// How much damage the shield can block before it breaks
    pub strength: f32,
    /// The maximum amount of strength a shield can hold
    pub max_strength: f32,
    /// How much power is consumed to generate the shield each second
    pub power_per_second: f32,
    /// How efficient the power usage is
    pub power_efficiency: f32,
    /// If this shield should be projecting or not.
    ///
    /// A shield can be disabled even if its strength is not 0.
    pub disabled: bool,
}

impl Shield {
    /// Returns true if this shield is currently active
    pub fn is_enabled(&self) -> bool {
        self.strength > f32::EPSILON && !self.disabled
    }

    /// Returns the strength value of this shield (or 0 if the shield is disabled)
    pub fn strength(&self) -> f32 {
        if self.disabled { 0.0 } else { self.strength }
    }

    /// Reduces the shield's strength based on the amount provided.
    ///
    /// The shield's strength cannot go below 0.0.
    pub fn take_damage(&mut self, amount: f32) {
        self.strength = (self.strength - amount).max(0.0);
    }
}

impl IdentifiableComponent for Shield {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shield"
    }
}

impl SyncableComponent for Shield {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

/// Things that should collide with shields should be put into this group
pub const SHIELD_COLLISION_GROUP: Group = Group::GROUP_3;

fn on_add_shield(
    q_rapier_entity_link: Query<&RapierContextEntityLink>,
    q_added_shield: Query<(Entity, &Shield, &ChildOf), Changed<Shield>>,
    mut commands: Commands,
) {
    for (ent, shield, parent) in q_added_shield.iter() {
        assert!(shield.radius > 0.0, "Shield radius cannot be <= 0.0!");

        let mut ecmds = commands.entity(ent);

        ecmds.insert((
            DespawnWithStructure,
            CollisionGroups::new(SHIELD_COLLISION_GROUP, SHIELD_COLLISION_GROUP),
            ColliderMassProperties::Mass(0.0),
            Sensor,
        ));

        if shield.is_enabled() {
            ecmds.insert(Collider::ball(shield.radius));
        }

        if let Ok(&pw) = q_rapier_entity_link.get(parent.parent()) {
            ecmds.insert(pw);
        }
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Shield>(app);

    app.add_systems(PostUpdate, on_add_shield.before(PhysicsSet::SyncBackend));

    app.register_type::<Shield>();
}
