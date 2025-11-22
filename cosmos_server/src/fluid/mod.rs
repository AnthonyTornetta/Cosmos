//! Server fluid logic

use bevy::prelude::*;
use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet, data::BlockData},
    events::block_events::BlockChangedMessage,
    fluid::data::{BlockFluidData, FluidItemData},
    registry::{Registry, identifiable::Identifiable},
    structure::Structure,
};
use interact_fluid::FluidInteractionSet;

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

pub mod interact_fluid;
mod register_blocks;
mod tank;

impl DefaultPersistentComponent for BlockFluidData {}
impl DefaultPersistentComponent for FluidItemData {}

fn on_place_tank(
    mut evr_changed_block: MessageReader<BlockChangedMessage>,
    mut q_structure: Query<&mut Structure>,
    q_has_data: Query<(), With<BlockFluidData>>,
    mut q_block_data: Query<&mut BlockData>,
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_changed_block.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };
        let coords = ev.block.coords();
        if structure.block_at(coords, &blocks).unlocalized_name() != "cosmos:tank" {
            continue;
        }

        // if structure.query_block_data(coords, &q_has_data).is_some() {
        //     continue;
        // }

        structure.insert_block_data(coords, BlockFluidData::NoFluid, &mut commands, &mut q_block_data, &q_has_data);
    }
}

pub(super) fn register(app: &mut App) {
    register_blocks::register(app);
    interact_fluid::register(app);
    tank::register(app);

    app.add_systems(
        FixedUpdate,
        on_place_tank
            .in_set(BlockMessagesSet::SendMessagesForThisFrame)
            .in_set(FluidInteractionSet::InteractWithFluidBlocks)
            .ambiguous_with(FluidInteractionSet::InteractWithFluidBlocks),
    );

    make_persistent::<FluidItemData>(app);
    make_persistent::<BlockFluidData>(app);
}
