use bevy::{
    app::{App, First, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, Parent},
    log::warn,
};
use cosmos_core::{
    block::{
        data::BlockData,
        storage::storage_blocks::{on_add_storage, PopulateBlockInventoryEvent},
    },
    inventory::Inventory,
    item::Item,
    registry::Registry,
    structure::{
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
        loading::StructureLoadingSet,
        Structure,
    },
};

use crate::structure::persistence::{
    chunk::{done_saving_block_data, ChunkLoadBlockDataEvent},
    BlockDataNeedsSavedThisIsStupidPleaseMakeThisAComponent, SuperDuperStupidGarbage,
};

// I can't get this to work no matter how many deffers I use.
// Wait till https://github.com/bevyengine/bevy/pull/9822 is released
// fn save_storage(
//     q_storage_blocks: Query<(&Parent, &Inventory, &BlockData), With<BlockDataNeedsSaved>>,
//     mut q_chunk: Query<&mut SerializedBlockData>,
// ) {
//     q_storage_blocks.for_each(|(parent, inventory, block_data)| {
//         let mut serialized_block_data = q_chunk
//             .get_mut(parent.get())
//             .expect("Block data's parent wasn't a chunk w/ SerializedBlockData???");

//         serialized_block_data.serialize_data(
//             ChunkBlockCoordinate::for_block_coordinate(block_data.block.coords()),
//             "cosmos:inventory",
//             inventory,
//         );
//     });
// }

fn save_storage(
    mut ev_reader: EventReader<BlockDataNeedsSavedThisIsStupidPleaseMakeThisAComponent>,
    q_storage_blocks: Query<(&Parent, &Inventory, &BlockData) /*With<BlockDataNeedsSaved>*/>,
    // mut q_chunk: Query<&mut SerializedBlockData>,
    mut garbage: ResMut<SuperDuperStupidGarbage>,
) {
    for ev in ev_reader.read() {
        if let Ok((parent, inventory, block_data)) = q_storage_blocks.get(ev.0) {
            let serialized_block_data = garbage
                .0
                .get_mut(&parent.get())
                .expect("Block data's parent wasn't a chunk w/ SerializedBlockData???");

            serialized_block_data.serialize_data(
                ChunkBlockCoordinate::for_block_coordinate(block_data.block.coords()),
                "cosmos:inventory",
                inventory,
            );
        }
    }
}

fn deserialize_storage(q_structure: Query<&Structure>, mut commands: Commands, mut ev_reader: EventReader<ChunkLoadBlockDataEvent>) {
    for ev in ev_reader.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let first = ev.chunk.first_structure_block();
        for (data_coord, serialized) in ev.data.iter() {
            let Some(inventory) = serialized.deserialize_data::<Inventory>("cosmos:inventory") else {
                continue;
            };

            let data_ent = structure
                .block_data(first + *data_coord)
                .expect("Missing data entity despite having data here");

            commands.entity(data_ent).insert(inventory);
        }
    }
}

fn populate_inventory(
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut commands: Commands,
    mut ev_reader: EventReader<PopulateBlockInventoryEvent>,
    items: Res<Registry<Item>>,
) {
    for ev in ev_reader.read() {
        let coords = ev.block.coords();

        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };

        let mut inv = Inventory::new(9 * 5, None);

        if let Some(item) = items.from_id("cosmos:stone") {
            inv.insert(item, 100);
        } else {
            warn!("Missing cosmos:stone?");
        }

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
                        block: ev.block,
                        structure_entity: ev.structure_entity,
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
        .add_systems(
            First,
            save_storage /* .after(apply_deferred_saving)*/
                .before(done_saving_block_data),
        )
        .add_systems(Update, deserialize_storage.in_set(StructureLoadingSet::LoadChunkData));
}
