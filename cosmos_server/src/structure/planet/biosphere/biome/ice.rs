//! Ice biome generation

use bevy::{
    ecs::system::{Res, ResMut},
    prelude::{App, IntoSystemConfigs},
    state::state::OnExit,
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::planet::generation::{biome::Biome, block_layers::BlockLayers},
};

use crate::state::GameState;

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
