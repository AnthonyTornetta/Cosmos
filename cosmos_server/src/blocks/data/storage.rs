//! Handles blocks that have inventories

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{EventReader, EventWriter},
        query::With,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
    },
    prelude::Event,
};
use cosmos_core::{
    block::{block_events::BlockEventsSet, data::BlockData, Block},
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    inventory::Inventory,
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    structure::{structure_block::StructureBlock, Structure},
};

use crate::{
    fluid::interact_fluid::FluidInteractionSet,
    persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded, LOADING_SCHEDULE},
};

#[derive(Event, Debug)]
/// Sent whenever an entity needs an inventory populated.
///
/// This should be populated by reading the block data on disk or creating a new inventory.
struct PopulateBlockInventoryEvent {
    /// The structure's entity
    pub structure_entity: Entity,
    /// The block
    pub block: StructureBlock,
}

/// Used to process the addition/removal of storage blocks to a structure.
///
/// Sends out the `PopulateBlockInventoryEvent` event when needed.
fn on_add_storage(
    mut q_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut ev_writer: EventWriter<PopulateBlockInventoryEvent>,
    mut q_block_data: Query<&mut BlockData>,
    mut params: BlockDataSystemParams,
    q_has_data: Query<(), With<Inventory>>,
) {
    if evr_block_changed.is_empty() {
        return;
    }

    let Some(block) = blocks.from_id("cosmos:storage") else {
        return;
    };

    for ev in evr_block_changed.read() {
        if ev.new_block == ev.old_block {
            continue;
        }

        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };

        if blocks.from_numeric_id(ev.old_block) == block {
            let coords = ev.block.coords();

            structure.remove_block_data::<Inventory>(coords, &mut params, &mut q_block_data, &q_has_data);
        }

        if blocks.from_numeric_id(ev.new_block) == block {
            ev_writer.send(PopulateBlockInventoryEvent {
                block: ev.block,
                structure_entity: ev.structure_entity,
            });
        }
    }
}

fn on_load_blueprint_storage(
    needs_blueprint_loaded_structure: Query<(Entity, &Structure), With<NeedsBlueprintLoaded>>,
    blocks: Res<Registry<Block>>,
    mut ev_writer: EventWriter<PopulateBlockInventoryEvent>,
) {
    for (structure_entity, structure) in needs_blueprint_loaded_structure.iter() {
        let Some(storage_block) = blocks.from_id("cosmos:storage") else {
            return;
        };

        for block in structure.all_blocks_iter(false) {
            if block.block_id(structure) == storage_block.id() {
                ev_writer.send(PopulateBlockInventoryEvent { block, structure_entity });
            }
        }
    }
}

fn populate_inventory(
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_inventory: Query<(), With<Inventory>>,
    mut params: BlockDataSystemParams,
    mut ev_reader: EventReader<PopulateBlockInventoryEvent>,
) {
    for ev in ev_reader.read() {
        let coords = ev.block.coords();

        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };

        structure.insert_block_data_with_entity(
            coords,
            |e| Inventory::new("Storage", 9 * 5, None, e),
            &mut params,
            &mut q_block_data,
            &q_has_inventory,
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_storage
                .in_set(BlockEventsSet::ProcessEvents)
                .ambiguous_with(FluidInteractionSet::InteractWithFluidBlocks),
            populate_inventory.in_set(BlockEventsSet::SendEventsForNextFrame),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    )
    .add_systems(
        LOADING_SCHEDULE,
        // Need structure to be populated first, thus `DoneLoadingBlueprints` instead of `DoLoadingBlueprints``
        on_load_blueprint_storage.in_set(LoadingBlueprintSystemSet::DoneLoadingBlueprints),
    )
    .add_event::<PopulateBlockInventoryEvent>();
}
