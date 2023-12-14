use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{EventReader, EventWriter},
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res},
    },
    hierarchy::{BuildChildren, Parent},
    log::warn,
};
use cosmos_core::{
    block::{
        data::{BlockData, BlockDataIdentifier},
        storage::storage_blocks::{on_add_storage, PopulateBlockInventoryEvent},
        Block,
    },
    inventory::Inventory,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        chunk::netty::SerializedBlockData,
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
        Structure,
    },
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
    q_storage_blocks.for_each(|(parent, inventory, block_data)| {
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
    mut commands: Commands,
    mut ev_reader: EventReader<PopulateBlockInventoryEvent>,
) {
    for ev in ev_reader.read() {
        let coords = ev.block.coords();

        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };

        let inv = Inventory::new("Storage", 9 * 5, None);

        if let Some(data_ent) = structure.block_data(coords) {
            // TODO:
            // If the BlockData was added the same frame as this from another system, this can cause the below if statement to be false,
            // which could lead to issues if 2 pieces of block data are added in the same frame.
            // This will need to be addressed in the future, as it will lead to data that holds nothing in blocks
            // A simple method would be to remove the error-prone counting and just send out mutable events every time this entity is changed
            // that would signal whether or not to remove this entity.
            //
            // For now, since there is only one possible type of data, this won't cause any issues (probably), but as soon as
            // more than just storage blocks exist, this will be a problem
            if let Ok(mut count) = q_block_data.get_mut(data_ent) {
                count.increment();
            }

            if let Some(mut ecmds) = commands.get_entity(data_ent) {
                ecmds.insert(inv);
            }
        } else {
            let Some(chunk_ent) = structure.chunk_entity(ChunkCoordinate::for_block_coordinate(coords)) else {
                warn!("Missing chunk entity but got block change event? How???");
                continue;
            };

            let data_ent = commands
                .spawn((
                    BlockData {
                        identifier: BlockDataIdentifier {
                            block: ev.block,
                            structure_entity: ev.structure_entity,
                        },
                        data_count: 1,
                    },
                    inv,
                ))
                .id();

            commands.entity(chunk_ent).add_child(data_ent);
            structure.set_block_data(coords, data_ent);
        };
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
