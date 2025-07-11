use crate::asset::materials::MaterialsSystemSet;
use bevy::prelude::*;
use cosmos_core::block::Block;
use cosmos_core::block::block_events::BlockEventsSet;
use cosmos_core::registry::Registry;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::state::GameState;

use super::{BlockMeshRegistry, MeshBuilder, MeshInformation};

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

#[derive(Component, Debug, Default, Clone, Copy)]
/// If this component is present on the blockdata of a block, then this block will be marked for
/// re-rendering when its block-data changes.
pub struct BlockDataRerenderOnChange;

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            StructureRenderingSet::MonitorBlockUpdates,
            StructureRenderingSet::BeginRendering,
            StructureRenderingSet::CustomRendering,
        )
            .chain()
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld)))
            .in_set(MaterialsSystemSet::RequestMaterialChanges)
            .after(BlockEventsSet::SendEventsForNextFrame),
    );

    app.add_systems(OnExit(GameState::PostLoading), fill_rendering_mode);

    chunk_rendering::register(app);
    monitor_needs_rerendered_chunks::register(app);

    app.init_resource::<BlockRenderingModes>();
}
