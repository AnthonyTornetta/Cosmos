use bevy::{ecs::event::Event, prelude::*};
use cosmos_core::structure::Structure;

use crate::structure::planet::biosphere::TGenerateChunkEvent;

#[derive(Component)]
pub struct NeedsGenerated;

/// T represents the event type to be generated
/// K represents the marker type for that specific biosphere
pub fn check_needs_generated_system<T: TGenerateChunkEvent + Event, K: Component>(
    mut commands: Commands,
    query: Query<&Structure, (With<NeedsGenerated>, With<K>)>,
    mut event_writer: EventWriter<T>,
) {
    for s in query.iter() {
        for chunk in s.all_chunks_iter() {
            event_writer.send(T::new(
                chunk.structure_x(),
                chunk.structure_y(),
                chunk.structure_z(),
                s.get_entity().unwrap(),
            ));
        }

        commands
            .entity(s.get_entity().unwrap())
            .remove::<NeedsGenerated>();
    }
}
