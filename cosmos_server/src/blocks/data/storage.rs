use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{EventReader, EventWriter},
        query::With,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
    },
    hierarchy::Parent,
};
use cosmos_core::{
    block::{
        data::BlockData,
        storage::storage_blocks::{on_add_storage, PopulateBlockInventoryEvent},
        Block,
    },
    events::block_events::BlockDataSystemParams,
    inventory::Inventory,
    registry::{identifiable::Identifiable, Registry},
    structure::{chunk::netty::SerializedBlockData, coordinates::ChunkBlockCoordinate, Structure},
};

use crate::{
    persistence::{
        loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded, LOADING_SCHEDULE},
        saving::SAVING_SCHEDULE,
    },
    structure::{
        persistence::{chunk::BlockDataSavingSet, BlockDataNeedsSaved},
        planet::chunk::SerializeChunkBlockDataSet,
    },
};

fn save_storage(
    q_storage_blocks: Query<(&Parent, &Inventory, &BlockData), With<BlockDataNeedsSaved>>,
    mut q_chunk: Query<&mut SerializedBlockData>,
) {
    q_storage_blocks.iter().for_each(|(parent, inventory, block_data)| {
        let mut serialized_block_data = q_chunk
            .get_mut(parent.get())
            .expect("Block data's parent didn't have SerializedBlockData???");

        serialized_block_data.serialize_data(
            ChunkBlockCoordinate::for_block_coordinate(block_data.identifier.block.coords()),
            "cosmos:inventory",
            inventory,
        );
    });
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
    app.add_systems(Update, populate_inventory.after(on_add_storage))
        .add_systems(SAVING_SCHEDULE, save_storage.in_set(BlockDataSavingSet::SaveBlockData))
        .add_systems(Update, save_storage.in_set(SerializeChunkBlockDataSet::Serialize))
        .add_systems(
            LOADING_SCHEDULE,
            // Need structure to be populated first, thus `DoneLoadingBlueprints` instead of `DoLoadingBlueprints``
            on_load_blueprint_storage.in_set(LoadingBlueprintSystemSet::DoneLoadingBlueprints),
        );
}
