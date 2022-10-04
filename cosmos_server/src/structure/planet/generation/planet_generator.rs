use bevy::{ecs::event::Event, prelude::*};
use cosmos_core::structure::structure::Structure;

use crate::structure::planet::biosphere::TGenerateChunkEvent;

#[derive(Component)]
pub struct NeedsGenerated;

pub fn check_needs_generated_system<T: TGenerateChunkEvent + Event>(
    mut commands: Commands,
    query: Query<&Structure, With<NeedsGenerated>>,
    mut event_writer: EventWriter<T>,
) {
    for s in query.iter() {
        for z in 0..s.chunks_length() {
            for y in 0..s.chunks_height() {
                for x in 0..s.chunks_width() {
                    event_writer.send(T::new(x, y, z, s.get_entity().unwrap()));
                }
            }
        }

        commands
            .entity(s.get_entity().unwrap())
            .remove::<NeedsGenerated>();
    }
}
