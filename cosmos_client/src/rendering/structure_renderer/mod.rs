use crate::asset::materials::{add_materials, remove_materials};
use crate::state::game_state::GameState;
use crate::structure::planet::unload_chunks_far_from_players;
use bevy::ecs::schedule::{IntoSystemSetConfigs, OnExit, SystemSet};
use bevy::prelude::{in_state, App, IntoSystemConfigs, Res, ResMut, Resource, Update};
use cosmos_core::block::Block;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::Registry;
use cosmos_core::structure::block_storage::BlockStorer;

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation};

pub mod chunk_rendering;
mod monitor_needs_rerendered_chunks;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RenderingMode {
    #[default]
    Standard,
    Both,
    Custom,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct BlockRenderingModes {
    blocks: Vec<RenderingMode>,
}

impl BlockRenderingModes {
    pub fn set_rendering_mode(&mut self, block: &Block, rendering_mode: RenderingMode) {
        let id = block.id();

        while self.blocks.len() <= id as usize {
            self.blocks.push(RenderingMode::Standard);
        }

        self.blocks[id as usize] = rendering_mode;
    }

    pub fn try_rendering_mode(&self, block_id: u16) -> Option<RenderingMode> {
        self.blocks.get(block_id as usize).copied()
    }

    pub fn rendering_mode(&self, block_id: u16) -> RenderingMode {
        self.blocks[block_id as usize]
    }
}

fn fill_rendering_mode(blocks: Res<Registry<Block>>, mut rendering_mode: ResMut<BlockRenderingModes>) {
    for block in blocks.iter() {
        if rendering_mode.try_rendering_mode(block.id()).is_none() {
            rendering_mode.set_rendering_mode(block, RenderingMode::Standard);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StructureRenderingSet {
    MonitorBlockUpdates,
    BeginRendering,
    CustomRendering,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            StructureRenderingSet::MonitorBlockUpdates,
            StructureRenderingSet::BeginRendering,
            StructureRenderingSet::CustomRendering,
        )
            .chain()
            .run_if(in_state(GameState::Playing))
            .before(unload_chunks_far_from_players)
            .before(remove_materials)
            .before(add_materials),
    );

    app.add_systems(OnExit(GameState::PostLoading), fill_rendering_mode);

    chunk_rendering::register(app);
    monitor_needs_rerendered_chunks::register(app);

    app.init_resource::<BlockRenderingModes>();
}
