//! Represents all the mining lasers on a structure

use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};

use super::StructureSystemsSet;
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
    /// How much energy is consumed per second mining
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
        let mut property = properties
            .iter()
            .copied()
            .reduce(|a, b: MiningLaserProperty| MiningLaserProperty {
                break_force: a.break_force + b.break_force,
                energy_per_second: a.energy_per_second + b.energy_per_second,
            })
            .unwrap_or_default();

        // Makes it cheaper early on (<500 drills), but more expensive the more drills you have
        property.energy_per_second = (property.energy_per_second / 20.0).powf(1.42);

        property
    }

    fn unlocalized_name() -> &'static str {
        "cosmos:mining_laser_system"
    }
}

fn name_mining_laser_system(mut commands: Commands, q_added: Query<Entity, Added<MiningLaserSystem>>) {
    for e in q_added.iter() {
        commands.entity(e).insert(Name::new("Plasma Drill System"));
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<MiningLaserSystem>().add_systems(
        FixedUpdate,
        name_mining_laser_system
            .ambiguous_with_all() // doesn't matter if this is 1-frame delayed
            .after(StructureSystemsSet::InitSystems),
    );
}
