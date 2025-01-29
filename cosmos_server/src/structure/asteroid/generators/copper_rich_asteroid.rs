use bevy::prelude::*;

use crate::structure::planet::biosphere::TemperatureRange;

use super::{
    standard_generator::{self, AsteroidOreEntry},
    AsteroidGeneratorComponent,
};

#[derive(Clone, Copy, Component, Default)]
struct CopperAsteroidMarker;

impl AsteroidGeneratorComponent for CopperAsteroidMarker {}

pub(super) fn register(app: &mut App) {
    standard_generator::register_standard_asteroid_generation::<CopperAsteroidMarker>(
        app,
        "cosmos:copper",
        TemperatureRange::new(0.0, 10000000000.0),
        vec![
            AsteroidOreEntry {
                ore: "cosmos:lead_ore",
                size: 0.03,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:iron_ore",
                size: 1.0,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:copper_ore",
                size: 1.0,
                rarity: 0.8,
            },
            AsteroidOreEntry {
                ore: "cosmos:uranium_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:sulfur_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:energite_crystal_ore",
                size: 0.05,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:photonium_crystal_ore",
                size: 0.05,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:gravitron_crystal_ore",
                size: 0.05,
                rarity: 0.3,
            },
        ],
        "cosmos:stone",
    );
}
