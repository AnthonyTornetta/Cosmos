//! Desert biome

use bevy::prelude::{App, EventWriter, OnExit, Res, ResMut};
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{coordinates::ChunkCoordinate, Structure},
};

use crate::{
    init::init_world::{Noise, ServerSeed},
    state::GameState,
    structure::planet::biosphere::biosphere_generation::BlockLayers,
};

use super::{biome_registry::RegisteredBiome, Biome};

/// Sandy without any features
pub struct OceanBiome {
    id: u16,
    unlocalized_name: String,
    block_layers: BlockLayers,
}

impl OceanBiome {
    /// Creates a new desert biome
    pub fn new(name: impl Into<String>, block_layers: BlockLayers) -> Self {
        Self {
            id: 0,
            block_layers,
            unlocalized_name: name.into(),
        }
    }
}

impl Biome for OceanBiome {
    fn block_layers(&self) -> &BlockLayers {
        &self.block_layers
    }

    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn generate_chunk_features(
        &self,
        _block_event_writer: &mut EventWriter<BlockChangedEvent>,
        _coords: ChunkCoordinate,
        _structure: &mut Structure,
        _location: &Location,
        _blocks: &Registry<Block>,
        _noise_generator: &Noise,
        _seed: &ServerSeed,
    ) {
    }
}

fn register_biome(mut registry: ResMut<Registry<RegisteredBiome>>, block_registry: Res<Registry<Block>>) {
    registry.register(RegisteredBiome::new(Box::new(OceanBiome::new(
        "cosmos:ocean",
        BlockLayers::default()
            .add_fixed_layer("cosmos:sand", &block_registry, 0)
            .expect("Sand missing")
            .add_fixed_layer("cosmos:stone", &block_registry, 4)
            .expect("Stone missing"),
    ))));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnExit(GameState::Loading), register_biome);
}
