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
                block_id: "cosmos:lead_ore",
                size: 0.3,
                rarity: 0.6,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:iron_ore",
                size: 1.0,
                rarity: 0.5,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:copper_ore",
                size: 1.0,
                rarity: 0.7,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:uranium_ore",
                size: 0.2,
                rarity: 0.5,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:sulfur_ore",
                size: 0.3,
                rarity: 0.8,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:lava",
                size: 1.0,
                rarity: 1.0,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:energite_crystal_ore",
                size: 0.2,
                rarity: 0.4,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:photonium_crystal_ore",
                size: 0.2,
                rarity: 0.4,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:gravitron_crystal_ore",
                size: 0.2,
                rarity: 0.4,
            },
        ],
        "cosmos:molten_stone",
    );
}
