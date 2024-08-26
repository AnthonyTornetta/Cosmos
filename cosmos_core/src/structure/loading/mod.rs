//! Handles the loading of structures

use crate::{
    netty::system_sets::NetworkingSystemsSet,
    structure::events::{ChunkSetEvent, StructureLoadedEvent},
};
use bevy::{
    ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
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
    for ev in event_reader.read() {
        let mut structure = structure_query.get_mut(ev.structure_entity).expect("Missing `Structure` component after got StructureLoadedEvent! Did you forget to add it? Make sure your system runs in `LoadingBlueprintSystemSet::DoLoadingBlueprints` or `LoadingBlueprintSystemSet::DoLoading`");

        if let Structure::Full(structure) = structure.as_mut() {
            structure.set_loaded();
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Systems responsible for the creation & population of a structure
pub enum StructureLoadingSet {
    /// Initially sets up the structure being loaded, such as creating the `Structure` component
    LoadStructure,
    /// Adds structure components that need to be present
    AddStructureComponents,
    /// Creates all entnties the chunks would have
    CreateChunkEntities,
    /// Loads the chunk from disk, and creates their serialized data.
    LoadChunkBlocks,
    /// Sets up the `BlockData` components used by block data
    InitializeChunkBlockData,
    /// Loads any chunk's block data
    LoadChunkData,
    /// Run once the structure is finished loaded. Used to notify other systems a chunk is ready to be processed
    StructureLoaded,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            StructureLoadingSet::LoadStructure,
            StructureLoadingSet::AddStructureComponents,
            StructureLoadingSet::CreateChunkEntities,
            StructureLoadingSet::LoadChunkBlocks,
            StructureLoadingSet::InitializeChunkBlockData,
            StructureLoadingSet::LoadChunkData,
            StructureLoadingSet::StructureLoaded,
        )
            .after(NetworkingSystemsSet::ProcessReceivedMessages)
            .chain(),
    );

    app.add_systems(
        Update,
        (
            listen_chunk_done_loading.in_set(StructureLoadingSet::LoadChunkData),
            set_structure_done_loading
                .ambiguous_with(StructureLoadingSet::StructureLoaded)
                .in_set(StructureLoadingSet::StructureLoaded),
        ),
    )
    .register_type::<ChunksNeedLoaded>();
}
