//! Handles the deserialization of storage

use bevy::{
    app::{App, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
    log::warn,
};

use crate::{
    block::data::{persistence::ChunkLoadBlockDataEvent, BlockData},
    inventory::Inventory,
    structure::{loading::StructureLoadingSet, Structure},
};

fn deserialize_storage(
    q_structure: Query<&Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut commands: Commands,
    mut ev_reader: EventReader<ChunkLoadBlockDataEvent>,
) {
    // for ev in ev_reader.read() {
    //     let Ok(structure) = q_structure.get(ev.structure_entity) else {
    //         warn!("No structure but tried to deserialize storage.");
    //         continue;
    //     };

    //     let first = ev.chunk.first_structure_block();
    //     for (data_coord, serialized) in ev.data.iter() {
    //         let Some(inventory) = serialized.deserialize_data::<Inventory>("cosmos:inventory") else {
    //             continue;
    //         };

    //         let data_ent = structure
    //             .block_data(first + *data_coord)
    //             .expect("Missing data entity despite having data here");

    //         commands.entity(data_ent).insert(inventory);
    //         q_block_data
    //             .get_mut(data_ent)
    //             .expect("Block data missing `BlockData` component!")
    //             .increment();
    //     }
    // }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, deserialize_storage.in_set(StructureLoadingSet::LoadChunkData));
}
