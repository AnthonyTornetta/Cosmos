//! Handles the loading of structures

use crate::structure::events::{ChunkSetEvent, StructureLoadedEvent};
use bevy::{
    prelude::{App, Commands, Component, EventReader, EventWriter, Query, Update, Without},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use super::{planet::Planet, Structure};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone, Copy)]
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
                commands.entity(ev.structure_entity).remove::<ChunksNeedLoaded>();

                event_writer.send(StructureLoadedEvent {
                    structure_entity: ev.structure_entity,
                });
            }
        }
    }
}

fn set_structure_done_loading(
    mut structure_query: Query<&mut Structure, Without<Planet>>,
    mut event_reader: EventReader<StructureLoadedEvent>,
) {
    for ent in event_reader.iter() {
        if let Ok(mut structure) = structure_query.get_mut(ent.structure_entity) {
            structure.set_all_loaded(true);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (listen_chunk_done_loading, set_structure_done_loading))
        .register_type::<ChunksNeedLoaded>();
}
