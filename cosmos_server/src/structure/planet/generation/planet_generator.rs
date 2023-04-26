//! Used to generate planets

use bevy::{ecs::event::Event, prelude::*};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONSF},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
        Structure,
    },
};

use crate::{state::GameState, structure::planet::biosphere::TGenerateChunkEvent};

#[derive(Component)]
/// This component will be in a planet's child entity if a chunk needs generated
///
/// This entity should be used as a flag, and is NOT the same as the chunk's entity
pub struct NeedsGenerated {
    chunk_coords: (usize, usize, usize),
    structure_entity: Entity,
}

/// T represents the event type to be generated
/// K represents the marker type for that specific biosphere
///
/// Use this to register your own planet generator
pub fn check_needs_generated_system<T: TGenerateChunkEvent + Event, K: Component>(
    mut commands: Commands,
    needs_generated_query: Query<(Entity, &NeedsGenerated)>,
    parent_query: Query<&Parent>,
    correct_type_query: Query<(), With<K>>,
    mut event_writer: EventWriter<T>,
) {
    for (entity, chunk) in needs_generated_query.iter() {
        let (cx, cy, cz) = chunk.chunk_coords;

        if let Ok(parent_entity) = parent_query.get(entity) {
            if correct_type_query.contains(parent_entity.get()) {
                event_writer.send(T::new(cx, cy, cz, chunk.structure_entity));

                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn generate_chunks_near_players(
    players: Query<&Location, With<Player>>,
    mut planets: Query<(&Location, &mut Structure, Entity), With<Planet>>,
    mut commands: Commands,
) {
    for player in players.iter() {
        let mut best_planet = None;
        let mut best_dist = f32::INFINITY;
        for (location, structure, entity) in planets.iter_mut() {
            let dist = location.distance_sqrd(player);
            if dist < best_dist {
                best_dist = dist;
                best_planet = Some((location, structure, entity));
            }
        }

        if let Some((location, mut best_planet, entity)) = best_planet {
            let player_relative_position: Vec3 = (*player - *location).into();
            let (px, py, pz) = best_planet.relative_coords_to_local_coords(
                player_relative_position.x,
                player_relative_position.y,
                player_relative_position.z,
            );

            let (px, py, pz) = (
                (px as f32 / CHUNK_DIMENSIONSF).floor() as i32,
                (py as f32 / CHUNK_DIMENSIONSF).floor() as i32,
                (pz as f32 / CHUNK_DIMENSIONSF).floor() as i32,
            );

            let mut chunks = vec![];

            for chunk in
                best_planet.chunk_iter((px - 2, py - 2, pz - 2), (px + 2, py + 2, pz + 2), true)
            {
                if let ChunkIteratorResult::EmptyChunk {
                    position: (x, y, z),
                } = chunk
                {
                    if !best_planet.is_chunk_loaded(x, y, z) {
                        chunks.push((x, y, z));
                    }
                }
            }

            for (x, y, z) in chunks {
                best_planet.set_chunk(Chunk::new(x, y, z));
                let needs_generated_flag = commands
                    .spawn(NeedsGenerated {
                        chunk_coords: (x, y, z),
                        structure_entity: entity,
                    })
                    .id();

                commands.entity(entity).add_child(needs_generated_flag);

                println!("FOUND CHUNK THAT NEEDS GENERATED @ {x} {y} {z}!");
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(generate_chunks_near_players.run_if(in_state(GameState::Playing)));
}
