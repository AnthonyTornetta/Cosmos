use bevy::{
    app::App,
    ecs::{
        schedule::OnEnter,
        system::{Res, ResMut},
    },
};

use cosmos_core::fluid::registry::Fluid;
use cosmos_core::{block::Block, registry::Registry};

use crate::{registry::sync_registry, state::GameState};

fn register_fluid_blocks(blocks: Res<Registry<Block>>, mut fluid_registry: ResMut<Registry<Fluid>>) {
    if blocks.contains("cosmos:water") {
        fluid_registry.register(Fluid::new("cosmos:water", 0.1));
    }

    if blocks.contains("cosmos:lava") {
        fluid_registry.register(Fluid::new("cosmos:lava", 0.1));
    }
}

pub(super) fn register(app: &mut App) {
    sync_registry::<Fluid>(app);

    app.add_systems(OnEnter(GameState::PostLoading), register_fluid_blocks);
}
