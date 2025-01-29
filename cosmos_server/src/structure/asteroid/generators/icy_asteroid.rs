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
        "cosmos:ice",
        TemperatureRange::new(0.0, 600.0),
        vec![
            AsteroidOreEntry {
                ore: "cosmos:lead_ore",
                size: 0.1,
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
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:uranium_ore",
                size: 0.2,
                rarity: 0.1,
            },
            AsteroidOreEntry {
                ore: "cosmos:sulfur_ore",
                size: 0.2,
                rarity: 0.2,
            },
            AsteroidOreEntry {
                ore: "cosmos:ice",
                size: 1.0,
                rarity: 1.0,
            },
            AsteroidOreEntry {
                ore: "cosmos:photonium_crystal_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:energite_crystal_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidOreEntry {
                ore: "cosmos:gravitron_crystal_ore",
                size: 0.1,
                rarity: 0.2,
            },
        ],
        "cosmos:stone",
    );
}
