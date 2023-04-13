//! Handles the loading of structures

use crate::structure::{
    events::{ChunkSetEvent, StructureLoadedEvent},
    Structure,
};
use bevy::{
    prelude::{Added, App, Commands, Component, Entity, EventReader, EventWriter, Query, Without},
    reflect::{FromReflect, Reflect},
};
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Reflect, FromReflect, Serialize, Deserialize, Clone, Copy)]
/// If a structure has this, not all its chunks have been filled out yet
/// and they need to be loaded
pub struct ChunksNeedLoaded {
    /// The number of chunks that need loaded
    pub amount_needed: usize,
}

fn listen_chunk_done_loading(
    mut event: EventReader<ChunkSetEvent>,
    mut query: Query<&mut ChunksNeedLoaded>,
    mut event_writer: EventWriter<StructureLoadedEvent>,
    mut commands: Commands,
) {
    for ev in event.iter() {
        let Ok(mut chunks_needed) = query.get_mut(ev.structure_entity) else {
            continue;
        };

        if chunks_needed.amount_needed != 0 {
            chunks_needed.amount_needed -= 1;

            if chunks_needed.amount_needed == 0 {
                commands
                    .entity(ev.structure_entity)
                    .remove::<ChunksNeedLoaded>();

                event_writer.send(StructureLoadedEvent {
                    structure_entity: ev.structure_entity,
                });
            }
        }
    }
}

fn listen_structure_added(
    query: Query<(Entity, &Structure), (Added<Structure>, Without<ChunksNeedLoaded>)>,
    mut commands: Commands,
) {
    for (entity, structure) in query.iter() {
        commands.entity(entity).insert(ChunksNeedLoaded {
            amount_needed: structure.all_chunks_iter(false).len(),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(listen_structure_added)
        .add_system(listen_chunk_done_loading)
        .register_type::<ChunksNeedLoaded>();
}
