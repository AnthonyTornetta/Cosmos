//! Ice biome generation

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    registry::Registry,
    state::GameState,
    structure::planet::generation::{biome::Biome, block_layers::BlockLayers},
};

use super::RegisterBiomesSet;

fn register_biome_ice(mut registry: ResMut<Registry<Biome>>, blocks: Res<Registry<Block>>) {
    registry.register(Biome::new(
        "cosmos:ice",
        BlockLayers::default()
            .add_fixed_layer("cosmos:ice", &blocks, 0)
            .expect("cosmos:ice missing!")
            .add_fixed_layer("cosmos:water", &blocks, 4)
            .expect("cosmos:water missing!"),
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        OnExit(GameState::Loading),
        register_biome_ice
            .in_set(RegisterBiomesSet::RegisterBiomes)
            .ambiguous_with(RegisterBiomesSet::RegisterBiomes),
    );
}
