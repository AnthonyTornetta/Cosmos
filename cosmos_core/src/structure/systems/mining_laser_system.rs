//! Represents all the mining lasers on a structure

use bevy::{prelude::*, reflect::Reflect};

use crate::{block::Block, registry::Registry};

use super::line_system::{add_line_system, LineBlocks, LineProperty, LinePropertyCalculator, LineSystem};

/// A ship system that stores information about the mining lasers
pub type MiningLaserSystem = LineSystem<MiningLaserProperty, MiningLaserPropertyCalculator>;

#[derive(Debug, Default, Reflect, Clone, Copy, PartialEq)]
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
}

fn register_laser_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<MiningLaserProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:plasma_drill") {
        cannon.insert(
            block,
            MiningLaserProperty {
                energy_per_second: 100.0,
                break_force: 1.0,
            },
        )
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_state: T) {
    add_line_system::<T, MiningLaserProperty, MiningLaserPropertyCalculator>(app, playing_state);

    app.add_systems(OnEnter(post_loading_state), register_laser_blocks);
}
