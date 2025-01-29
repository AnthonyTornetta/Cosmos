use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashMap};
use cosmos_core::{
    block::{block_rotation::BlockRotation, Block},
    physics::location::Location,
    registry::ReadOnlyRegistry,
    state::GameState,
    structure::{
        block_storage::BlockStorer,
        chunk::Chunk,
        coordinates::{BlockCoordinate, ChunkBlockCoordinate, ChunkCoordinate},
        Structure,
    },
    utils::timer::UtilsTimer,
};
use noise::NoiseFn;

use crate::{
    init::init_world::ReadOnlyNoise,
    structure::{
        asteroid::generator::{AsteroidGenerationSet, GenerateAsteroidEvent, GeneratingAsteroids},
        planet::biosphere::TemperatureRange,
    },
};

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
        vec![AsteroidOreEntry {
            ore: "cosmos:energite_crystal_ore",
            size: 1.0,
            rarity: 1.0,
        }],
        "cosmos:molten_stone",
    );
}
