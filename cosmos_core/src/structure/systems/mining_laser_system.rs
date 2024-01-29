//! Represents all the mining lasers on a structure

use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use super::{
    line_system::{LineProperty, LinePropertyCalculator, LineSystem},
    sync::SyncableSystem,
};

/// A ship system that stores information about the mining lasers
pub type MiningLaserSystem = LineSystem<MiningLaserProperty, MiningLaserPropertyCalculator>;

impl SyncableSystem for MiningLaserSystem {}

#[derive(Debug, Default, Reflect, Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Every block that is a mining laser should have this property
pub struct MiningLaserProperty {
    /// How much energy is consumed per shot
    pub energy_per_second: f32,
    /// The breaking force this block has
    ///
    /// Base is 1.0
    pub break_force: f32,
}

impl LineProperty for MiningLaserProperty {}

#[derive(Default, Reflect, Debug)]
/// Used internally by mining laser system, but must be public for compiler to be happy.
///
/// A simple strategy pattern that is never initialized
pub struct MiningLaserPropertyCalculator;

impl LinePropertyCalculator<MiningLaserProperty> for MiningLaserPropertyCalculator {
    fn calculate_property(properties: &[MiningLaserProperty]) -> MiningLaserProperty {
        properties
            .iter()
            .copied()
            .reduce(|a, b: MiningLaserProperty| MiningLaserProperty {
                break_force: a.break_force + b.break_force,
                energy_per_second: a.energy_per_second + b.energy_per_second,
            })
            .unwrap_or_default()
    }

    fn unlocalized_name() -> &'static str {
        "cosmos:mining_laser_system"
    }
}
