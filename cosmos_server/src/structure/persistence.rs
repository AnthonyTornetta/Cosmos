use bevy::prelude::*;
use cosmos_core::structure::{structure_iterator::ChunkIteratorResult, ChunkInitEvent, Structure};

// I hate this

// The only way to prevent issues with events is to delay the sending of the chunk init events by 2 frames,
// so two events are needed to do this. This is really horrible, but the only way I can think of
// to get this to work ;(
pub(crate) struct DelayedStructureLoadEvent(pub Entity);
struct EvenMoreDelayedStructureLoadEvent(Entity);

fn delayed_structure_event(
    mut event_reader: EventReader<DelayedStructureLoadEvent>,
    mut event_writer: EventWriter<EvenMoreDelayedStructureLoadEvent>,
) {
    for ev in event_reader.iter() {
        event_writer.send(EvenMoreDelayedStructureLoadEvent(ev.0));
    }
}

fn even_more_delayed_structure_event(
    mut event_reader: EventReader<EvenMoreDelayedStructureLoadEvent>,
    mut chunk_set_event_writer: EventWriter<ChunkInitEvent>,
    query: Query<&Structure>,
) {
    for ev in event_reader.iter() {
        if let Ok(structure) = query.get(ev.0) {
            for res in structure.all_chunks_iter(false) {
                // This will always be true because include_empty is false
                if let ChunkIteratorResult::FilledChunk {
                    position: (x, y, z),
                    chunk: _,
                } = res
                {
                    chunk_set_event_writer.send(ChunkInitEvent {
                        structure_entity: ev.0,
                        x,
                        y,
                        z,
                    });
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(even_more_delayed_structure_event.in_base_set(CoreSet::PreUpdate))
        // After to ensure 1 frame delay
        .add_system(delayed_structure_event.after(even_more_delayed_structure_event))
        .add_event::<DelayedStructureLoadEvent>()
        .add_event::<EvenMoreDelayedStructureLoadEvent>();
}
