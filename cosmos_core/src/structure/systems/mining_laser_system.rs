//! Represents all the mining lasers on a structure

use std::time::Duration;

use bevy::{prelude::*, reflect::Reflect};

use crate::{block::Block, registry::Registry};

use super::line_system::{add_line_system, LineBlocks, LineProperty, LinePropertyCalculator};

#[derive(Debug, Default, Reflect, Clone, Copy, PartialEq)]
/// Every block that is a mining laser should have this property
pub struct MiningLaserProperty {
    /// How much energy is consumed per shot
    pub energy_per_second: f32,
    /// The duration it takes to break a block
    pub break_speed: Duration,
}

impl LineProperty for MiningLaserProperty {}

#[derive(Default, Reflect, Debug)]
struct MiningLaserPropertyCalculator;

impl LinePropertyCalculator<MiningLaserProperty> for MiningLaserPropertyCalculator {
    fn calculate_property(properties: &[MiningLaserProperty]) -> MiningLaserProperty {
        properties
            .iter()
            .copied()
            .reduce(|a, b: MiningLaserProperty| MiningLaserProperty {
                break_speed: a.break_speed + b.break_speed,
                energy_per_second: a.energy_per_second + b.energy_per_second,
            })
            .unwrap_or_default()
    }
}

fn register_laser_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<MiningLaserProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:laser_cannon") {
        cannon.insert(
            block,
            MiningLaserProperty {
                energy_per_second: 100.0,
                break_speed: Duration::from_secs(10),
            },
        )
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_state: T) {
    add_line_system::<T, MiningLaserProperty, MiningLaserPropertyCalculator>(app, playing_state);

    app.add_systems(OnEnter(post_loading_state), register_laser_blocks);
}
