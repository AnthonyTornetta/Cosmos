//! Server fluid logic

use bevy::{
    app::{App, Update},
    prelude::{EventReader, IntoSystemConfigs, Query, Res, With},
};
use cosmos_core::{
    block::{block_events::BlockEventsSet, data::BlockData, Block},
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    fluid::data::{BlockFluidData, FluidItemData, FluidTankBlock},
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};
use interact_fluid::FluidInteractionSet;

use crate::persistence::make_persistent::{make_persistent, PersistentComponent};

pub mod interact_fluid;
mod register_blocks;
mod tank;

impl PersistentComponent for BlockFluidData {}
impl PersistentComponent for FluidItemData {}

fn on_place_tank(
    mut evr_changed_block: EventReader<BlockChangedEvent>,
    mut q_structure: Query<&mut Structure>,
    q_has_data: Query<(), With<BlockFluidData>>,
    mut q_block_data: Query<&mut BlockData>,
    mut bs_params: BlockDataSystemParams,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_changed_block.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };
        let coords = ev.block.coords();
        if structure.block_at(coords, &blocks).unlocalized_name() != "cosmos:tank" {
            continue;
        }

        // if structure.query_block_data(coords, &q_has_data).is_some() {
        //     continue;
        // }

        structure.insert_block_data(coords, BlockFluidData::NoFluid, &mut bs_params, &mut q_block_data, &q_has_data);
    }
}

pub(super) fn register(app: &mut App) {
    register_blocks::register(app);
    interact_fluid::register(app);
    tank::register(app);

    app.add_systems(
        Update,
        on_place_tank
            .in_set(NetworkingSystemsSet::Between)
            .in_set(BlockEventsSet::SendEventsForThisFrame)
            .in_set(FluidInteractionSet::InteractWithFluidBlocks)
            .ambiguous_with(FluidInteractionSet::InteractWithFluidBlocks),
    );

    make_persistent::<FluidItemData>(app);
    make_persistent::<BlockFluidData>(app);
}
