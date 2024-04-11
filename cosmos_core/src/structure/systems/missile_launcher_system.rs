//! Represents all the missile launchers on this structure

use std::time::Duration;

use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query},
    },
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, SyncableComponent};

use super::{
    line_system::{LineProperty, LinePropertyCalculator, LineSystem},
    sync::SyncableSystem,
};

/// A ship system that stores information about the missile cannons
///
/// See [`SystemCooldown`] for the missile cannon's duration
pub type MissileLauncherSystem = LineSystem<MissileLauncherProperty, MissileLauncherCalculator>;

impl SyncableSystem for MissileLauncherSystem {}

#[derive(Default, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
/// Every block that is a missile cannon should have this property
pub struct MissileLauncherProperty {
    /// How much energy is consumed per shot
    pub energy_per_shot: f32,
}

impl LineProperty for MissileLauncherProperty {}

#[derive(Debug)]
/// Used internally by missile cannon system, but must be public for compiler to be happy.
///
/// A simple strategy pattern that is never initialized
pub struct MissileLauncherCalculator;

impl LinePropertyCalculator<MissileLauncherProperty> for MissileLauncherCalculator {
    fn calculate_property(properties: &[MissileLauncherProperty]) -> MissileLauncherProperty {
        properties
            .iter()
            .copied()
            .reduce(|a, b| MissileLauncherProperty {
                energy_per_shot: a.energy_per_shot + b.energy_per_shot,
            })
            .unwrap_or_default()
    }

    fn unlocalized_name() -> &'static str {
        "cosmos:missile_launcher_system"
    }
}

#[derive(Debug, Serialize, Deserialize, Component, Clone, Copy, Default)]
/// Tracks the current target the missile system is targetting
pub struct MissileLauncherFocus {
    focusing_entity: Option<Entity>,
    time_focused: Duration,
}

#[derive(Debug, Serialize, Deserialize, Component, Clone, Copy)]
/// Prefers focusing this entity if there are many potential candidates
pub struct MissileLauncherPreferredFocus(Entity);

impl SyncableComponent for MissileLauncherPreferredFocus {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:missile_launcher_focus"
    }

    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ClientAuthoritative
    }
}

fn add_focus_to_new_missile_system(mut commands: Commands, q_added_missile_launcher_system: Query<Entity, Added<MissileLauncherSystem>>) {
    for ent in &q_added_missile_launcher_system {
        commands.entity(ent).insert(MissileLauncherFocus::default());
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<MissileLauncherPreferredFocus>(app);

    app.add_systems(Update, add_focus_to_new_missile_system);
}
