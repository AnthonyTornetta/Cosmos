//! Represents all the laser cannons on this structure

use std::time::Duration;

use bevy::{prelude::*, reflect::Reflect};
use serde::{Deserialize, Serialize};

use super::{
    line_system::{LineProperty, LinePropertyCalculator, LineSystem},
    sync::SyncableSystem,
};

/// A ship system that stores information about the laser cannons
///
/// See [`SystemCooldown`] for the laser cannon's duration
pub type LaserCannonSystem = LineSystem<LaserCannonProperty, LaserCannonCalculator>;

impl SyncableSystem for LaserCannonSystem {}

#[derive(Default, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
/// Every block that is a laser cannon should have this property
pub struct LaserCannonProperty {
    /// How much energy is consumed per shot
    pub energy_per_shot: f32,
}

impl LineProperty for LaserCannonProperty {}

#[derive(Debug)]
/// Used internally by laser cannon system, but must be public for compiler to be happy.
///
/// A simple strategy pattern that is never initialized
pub struct LaserCannonCalculator;

impl LinePropertyCalculator<LaserCannonProperty> for LaserCannonCalculator {
    fn calculate_property(properties: &[LaserCannonProperty]) -> LaserCannonProperty {
        properties
            .iter()
            .copied()
            .reduce(|a, b| LaserCannonProperty {
                energy_per_shot: a.energy_per_shot + b.energy_per_shot,
            })
            .unwrap_or_default()
    }

    fn unlocalized_name() -> &'static str {
        "cosmos:laser_cannon_system"
    }
}

#[derive(Component, Default, Reflect)]
/// Represents all the laser cannons that are within this structure
pub struct SystemCooldown {
    /// The time since this system was last fired.
    pub last_use_time: f32,
    /// How long the cooldown should be
    pub cooldown_time: Duration,
}
