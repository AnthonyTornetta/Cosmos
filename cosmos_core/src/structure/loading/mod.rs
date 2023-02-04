use crate::structure::{
    events::{ChunkSetEvent, StructureLoadedEvent},
    Structure,
};
use bevy::prelude::{Added, App, Commands, Component, Entity, EventReader, EventWriter, Query};

#[derive(Component)]
pub struct ChunksNeedLoaded {
    pub amount_needed: usize,
}

fn listen_chunk_done_loading(
    mut event: EventReader<ChunkSetEvent>,
    mut query: Query<&mut ChunksNeedLoaded>,
    mut event_writer: EventWriter<StructureLoadedEvent>,
    mut commands: Commands,
) {
    for ev in event.iter() {
        if let Ok(mut chunks_needed) = query.get_mut(ev.structure_entity) {
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
    query: Query<(Entity, &Structure), Added<Structure>>,
    mut commands: Commands,
) {
    for (entity, structure) in query.iter() {
        commands.entity(entity).insert(ChunksNeedLoaded {
            amount_needed: structure.all_chunks_iter().len(),
        });
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(listen_structure_added)
        .add_system(listen_chunk_done_loading);
}
