//! Contains the serialized versions of block data shared between the client + server

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{Event, EventReader},
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
    hierarchy::BuildChildren,
    log::warn,
};

use crate::structure::{
    chunk::netty::SerializedChunkBlockData, coordinates::ChunkCoordinate, loading::StructureLoadingSet, structure_block::StructureBlock,
    Structure,
};

use super::{BlockData, BlockDataIdentifier};

#[derive(Event, Debug)]
/// This event is created whenever a chunk needs to load block data
pub struct ChunkLoadBlockDataEvent {
    /// The serialized block data
    pub data: SerializedChunkBlockData,
    /// The chunk's coordinates
    pub chunk: ChunkCoordinate,
    /// The structure's entity
    pub structure_entity: Entity,
}

fn add_chunk_data(mut ev_reader: EventReader<ChunkLoadBlockDataEvent>, mut commands: Commands, mut q_structure: Query<&mut Structure>) {
    for ev in ev_reader.read() {
        println!("GOT EVENT - ADDING BLOCK DATA!!!!");

        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            warn!("Missing structure for block data");
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
                        identifier: BlockDataIdentifier {
                            block: StructureBlock::new(coords),
                            structure_entity: ev.structure_entity,
                        },
                        data_count: 0,
                    },))
                    .id();

                structure.set_block_data(coords, data_ent);
            }
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChunkLoadBlockDataEvent>();

    app.add_systems(Update, add_chunk_data.in_set(StructureLoadingSet::InitializeChunkBlockData));
}
