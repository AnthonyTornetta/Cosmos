//! Handles the loading of structures

use crate::structure::events::{ChunkSetEvent, StructureLoadedEvent};
use bevy::{
    ecs::schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
    log::warn,
    prelude::{App, Commands, Component, EventReader, EventWriter, Query, Update},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use super::Structure;

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
    for ev in event.read() {
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

fn set_structure_done_loading(mut structure_query: Query<&mut Structure>, mut event_reader: EventReader<StructureLoadedEvent>) {
    for ent in event_reader.read() {
        println!("Got entity in reader!");
        if let Ok(mut structure) = structure_query.get_mut(ent.structure_entity) {
            if let Structure::Full(structure) = structure.as_mut() {
                structure.set_loaded();
            } else {
                warn!("Not full.");
            }
        } else {
            panic!("Missing `Structure` component after got StructureLoadedEvent! Did you forget to add it? Make sure your system runs in `LoadingBlueprintSystemSet::DoLoadingBlueprints` or `LoadingBlueprintSystemSet::DoLoading`");
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            StructureLoadingSet::LoadStructure,
            StructureLoadingSet::FlushStructureComponents,
            StructureLoadingSet::CreateChunkEntities,
            StructureLoadingSet::FlushChunkComponents,
            StructureLoadingSet::LoadChunkDataBase,
            StructureLoadingSet::FlushBlockDataBase,
            StructureLoadingSet::LoadChunkData,
            StructureLoadingSet::StructureLoaded,
        )
            .chain(),
    )
    .add_systems(Update, apply_deferred.in_set(StructureLoadingSet::FlushStructureComponents))
    .add_systems(Update, apply_deferred.in_set(StructureLoadingSet::FlushChunkComponents))
    .add_systems(Update, apply_deferred.in_set(StructureLoadingSet::FlushBlockDataBase));

    app.add_systems(
        Update,
        (
            listen_chunk_done_loading.in_set(StructureLoadingSet::LoadChunkData),
            set_structure_done_loading.in_set(StructureLoadingSet::StructureLoaded),
        ),
    )
    .register_type::<ChunksNeedLoaded>();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StructureLoadingSet {
    LoadStructure,
    /// apply_deferred
    FlushStructureComponents,
    CreateChunkEntities,
    /// apply_deferred
    FlushChunkComponents,
    LoadChunkDataBase,
    /// apply_deferred
    FlushBlockDataBase,
    LoadChunkData,
    StructureLoaded,
}
