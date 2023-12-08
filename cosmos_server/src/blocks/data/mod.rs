use bevy::{
    app::{App, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
    hierarchy::BuildChildren,
    log::warn,
};
use cosmos_core::{
    block::data::BlockData,
    structure::{loading::StructureLoadingSet, structure_block::StructureBlock, Structure},
};

use crate::structure::persistence::chunk::ChunkLoadBlockDataEvent;

mod storage;

fn add_chunk_data(mut ev_reader: EventReader<ChunkLoadBlockDataEvent>, mut commands: Commands, mut q_structure: Query<&mut Structure>) {
    for ev in ev_reader.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };
        let Some(chunk_ent) = structure.chunk_entity(ev.chunk) else {
            warn!("A chunk had data but there was no chunk entity.");
            continue;
        };

        let first_block_coord = ev.chunk.first_structure_block();

        commands.entity(chunk_ent).with_children(|chunk_ecmds| {
            for (coord, _) in ev.data.iter() {
                let coords = first_block_coord + *coord;

                let data_ent = chunk_ecmds
                    .spawn((BlockData {
                        block: StructureBlock::new(coords),
                        structure_entity: ev.structure_entity,
                        data_count: 0,
                    },))
                    .id();

                structure.set_block_data(coords, data_ent);
            }
        });
    }
}

pub(super) fn register(app: &mut App) {
    storage::register(app);

    app.add_systems(Update, add_chunk_data.in_set(StructureLoadingSet::LoadChunkDataBase));
}
