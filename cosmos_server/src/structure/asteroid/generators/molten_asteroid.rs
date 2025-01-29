use bevy::prelude::*;

use crate::structure::planet::biosphere::TemperatureRange;

use super::{
    standard_generator::{self, AsteroidOreEntry},
    AsteroidGeneratorComponent,
};

#[derive(Clone, Copy, Component, Default)]
struct MoltenAsteroidMarker;

impl AsteroidGeneratorComponent for MoltenAsteroidMarker {}

pub(super) fn register(app: &mut App) {
    standard_generator::register_standard_asteroid_generation::<MoltenAsteroidMarker>(
        app,
        "cosmos:molten",
        TemperatureRange::new(600.0, 10000000000.0),
        vec![
            AsteroidOreEntry {
                ore: "cosmos:lava",
                size: 1.0,
                rarity: 1.0,
            },
            AsteroidOreEntry {
                ore: "cosmos:energite_crystal_ore",
                size: 0.1,
                rarity: 0.5,
            },
        ],
        "cosmos:molten_stone",
    );
}
