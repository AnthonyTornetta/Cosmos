//! Desert biome

use bevy::prelude::*;
use cosmos_core::{block::Block, registry::Registry, state::GameState, structure::planet::generation::block_layers::BlockLayers};

use super::{Biome, RegisterBiomesSet};

fn register_biome(mut registry: ResMut<Registry<Biome>>, block_registry: Res<Registry<Block>>) {
    registry.register(Biome::new(
        "cosmos:ocean",
        BlockLayers::default()
            .add_fixed_layer("cosmos:sand", &block_registry, 0)
            .expect("Sand missing")
            .add_fixed_layer("cosmos:stone", &block_registry, 4)
            .expect("Stone missing"),
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        OnExit(GameState::Loading),
        register_biome
            .in_set(RegisterBiomesSet::RegisterBiomes)
            .ambiguous_with(RegisterBiomesSet::RegisterBiomes),
    );
}
