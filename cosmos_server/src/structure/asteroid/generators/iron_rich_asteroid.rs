use bevy::prelude::*;

use crate::structure::planet::biosphere::TemperatureRange;

use super::{
    AsteroidGeneratorComponent,
    standard_generator::{self, AsteroidBlockEntry},
};

#[derive(Clone, Copy, Component, Default)]
struct IronAsteroidMarker;

impl AsteroidGeneratorComponent for IronAsteroidMarker {}

pub(super) fn register(app: &mut App) {
    standard_generator::register_standard_asteroid_generation::<IronAsteroidMarker>(
        app,
        "cosmos:iron",
        TemperatureRange::new(0.0, 10000000000.0),
        vec![
            AsteroidBlockEntry {
                block_id: "cosmos:lead_ore",
                size: 0.03,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:iron_ore",
                size: 1.0,
                rarity: 0.8,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:copper_ore",
                size: 1.0,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:uranium_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:sulfur_ore",
                size: 0.1,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:energite_crystal_ore",
                size: 0.2,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:photonium_crystal_ore",
                size: 0.2,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:gravitron_crystal_ore",
                size: 0.05,
                rarity: 0.3,
            },
            AsteroidBlockEntry {
                block_id: "cosmos:sand",
                size: 0.3,
                rarity: 0.4,
            },
        ],
        "cosmos:stone",
    );
}
