//! Represents all the laser cannons on this structure

use std::time::Duration;

use bevy::{prelude::*, reflect::Reflect};

use crate::{block::Block, registry::Registry};

use super::line_system::{add_line_system, LineBlocks, LineProperty, LinePropertyCalculator, LineSystem};

#[derive(Default, Reflect, Clone, Copy)]
/// Every block that is a laser cannon should have this property
pub struct LaserCannonProperty {
    /// How much energy is consumed per shot
    pub energy_per_shot: f32,
}

impl LineProperty for LaserCannonProperty {}

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
}

#[derive(Component, Default, Reflect)]
/// Represents all the laser cannons that are within this structure
pub struct SystemCooldown {
    /// The time since this system was last fired.
    pub last_use_time: f32,
    /// How long the cooldown should be
    pub cooldown_time: Duration,
}

/// A ship system that stores information about the laser cannons
///
/// See [`SystemCooldown`] for the laser cannon's duration
pub type LaserCannonSystem = LineSystem<LaserCannonProperty, LaserCannonCalculator>;

fn on_add_laser(mut commands: Commands, query: Query<Entity, Added<LaserCannonSystem>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(SystemCooldown {
            cooldown_time: Duration::from_millis(200),
            ..Default::default()
        });
    }
}

fn register_laser_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<LaserCannonProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:laser_cannon") {
        cannon.insert(block, LaserCannonProperty { energy_per_shot: 100.0 })
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T, playing_state: T) {
    add_line_system::<T, LaserCannonProperty, LaserCannonCalculator>(app, playing_state);

    app.add_systems(OnEnter(post_loading_state), register_laser_blocks)
        .add_systems(Update, on_add_laser);
}
