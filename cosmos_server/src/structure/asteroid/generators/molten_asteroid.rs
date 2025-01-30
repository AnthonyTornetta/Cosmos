use bevy::prelude::*;

use crate::structure::planet::biosphere::TemperatureRange;

use super::{
    standard_generator::{self, AsteroidBlockEntry},
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
            AsteroidBlockEntry {
                ore: "cosmos:lead_ore",
                size: 0.1,
                rarity: 0.4,
            },
            AsteroidBlockEntry {
                ore: "cosmos:iron_ore",
                size: 1.0,
                rarity: 0.5,
            },
            AsteroidBlockEntry {
                ore: "cosmos:copper_ore",
                size: 1.0,
                rarity: 0.5,
            },
            AsteroidBlockEntry {
                ore: "cosmos:uranium_ore",
                size: 0.2,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                ore: "cosmos:sulfur_ore",
                size: 0.2,
                rarity: 0.4,
            },
            AsteroidBlockEntry {
                ore: "cosmos:lava",
                size: 1.0,
                rarity: 1.0,
            },
            AsteroidBlockEntry {
                ore: "cosmos:energite_crystal_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                ore: "cosmos:photonium_crystal_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                ore: "cosmos:gravitron_crystal_ore",
                size: 0.1,
                rarity: 0.3,
            },
        ],
        "cosmos:molten_stone",
    );
}
