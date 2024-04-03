//! Represents all the missile launchers on this structure

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

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
