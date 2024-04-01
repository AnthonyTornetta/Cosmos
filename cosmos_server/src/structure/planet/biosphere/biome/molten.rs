//! Molten biome generation

use bevy::{
    ecs::{
        schedule::OnExit,
        system::{Res, ResMut},
    },
    prelude::App,
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::planet::generation::{biome::Biome, block_layers::BlockLayers},
};

use crate::state::GameState;

fn register_biome_molten(mut registry: ResMut<Registry<Biome>>, blocks: Res<Registry<Block>>) {
    registry.register(Biome::new(
        "cosmos:molten",
        BlockLayers::default()
            .add_fixed_layer("cosmos:molten_stone", &blocks, 0)
            .expect("cosmos:molten_stone missing!"),
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnExit(GameState::Loading), register_biome_molten);
}
