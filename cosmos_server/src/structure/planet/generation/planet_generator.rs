//! Used to generate planets

use bevy::{ecs::event::Event, prelude::*};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        chunk::{Chunk, ChunkEntity},
        planet::Planet,
        Structure,
    },
};

use crate::structure::planet::biosphere::TGenerateChunkEvent;

#[derive(Component)]
/// This component will be present if a planet needs generated
pub struct NeedsGenerated;

struct ChunkGenerated;

/// T represents the event type to be generated
/// K represents the marker type for that specific biosphere
///
/// Use this to register your own planet generator
pub fn check_needs_generated_system<T: TGenerateChunkEvent + Event, K: Component>(
    mut commands: Commands,
    query: Query<(Entity, &ChunkEntity), (With<NeedsGenerated>, With<K>)>,
    mut event_writer: EventWriter<T>,
) {
    for (entity, chunk) in query.iter() {
        let (cx, cy, cz) = chunk.chunk_location;

        event_writer.send(T::new(cx, cy, cz, chunk.structure_entity));

        commands.entity(entity).remove::<NeedsGenerated>();
    }
}

fn generate_chunks_near_players(
    players: Query<&Location, With<Player>>,
    mut planets: Query<(&Location, &mut Structure), With<Planet>>,
) {
    for player in players.iter() {
        let mut best_planet = None;
        let mut best_dist = f32::INFINITY;
        for (location, structure) in planets.iter_mut() {
            let dist = location.distance_sqrd(player);
            if dist < best_dist {
                best_dist = dist;
                best_planet = Some(structure);
            }
        }

        if let Some(mut best_planet) = best_planet {
            for z in 0..best_planet.chunks_length() {
                for y in 0..best_planet.chunks_height() {
                    for x in 0..best_planet.chunks_width() {
                        best_planet.set_chunk(Chunk::new(x, y, z));
                    }
                }
            }
        }
    }
}
