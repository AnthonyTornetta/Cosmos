//! Used to generate planets

use bevy::{ecs::event::Event, prelude::*};
use cosmos_core::structure::{structure_iterator::ChunkIteratorResult, Structure};

use crate::structure::planet::biosphere::TGenerateChunkEvent;

#[derive(Component)]
/// This component will be present if a planet needs generated
pub struct NeedsGenerated;

/// T represents the event type to be generated
/// K represents the marker type for that specific biosphere
///
/// Use this to register your own planet generator
pub fn check_needs_generated_system<T: TGenerateChunkEvent + Event, K: Component>(
    mut commands: Commands,
    query: Query<&Structure, (With<NeedsGenerated>, With<K>)>,
    mut event_writer: EventWriter<T>,
) {
    for s in query.iter() {
        for chunk in s.all_chunks_iter(true) {
            let (cx, cy, cz) = match chunk {
                ChunkIteratorResult::EmptyChunk { position } => position,
                ChunkIteratorResult::FilledChunk { position, chunk: _ } => position,
            };

            event_writer.send(T::new(cx, cy, cz, s.get_entity().unwrap()));
        }

        commands
            .entity(s.get_entity().unwrap())
            .remove::<NeedsGenerated>();
    }
}
