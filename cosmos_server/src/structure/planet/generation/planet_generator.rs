//! Used to generate planets

use std::sync::{
    atomic::{AtomicI32, Ordering},
    Mutex,
};

use bevy::{
    ecs::event::Event,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{
        cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannel,
        NoSendEntity,
    },
    physics::location::Location,
    structure::{
        chunk::CHUNK_DIMENSIONSF, planet::Planet, structure_iterator::ChunkIteratorResult,
        ChunkState, Structure,
    },
    utils::timer::UtilsTimer,
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    persistence::{
        saving::{NeedsSaved, NeedsUnloaded},
        EntityId, SaveFileIdentifier,
    },
    state::GameState,
    structure::planet::{
        biosphere::TGenerateChunkEvent, chunk::SaveChunk, persistence::ChunkNeedsPopulated,
    },
};

#[derive(Component)]
/// This component will be in a planet's child entity if a chunk needs generated
///
/// This entity should be used as a flag, and is NOT the same as the chunk's entity
pub struct ChunkNeedsGenerated {
    /// The chunk's coordinates in the structure
    pub chunk_coords: (usize, usize, usize),
    /// The structure's entity
    pub structure_entity: Entity,
}

/// T represents the event type to be generated
/// K represents the marker type for that specific biosphere
///
/// Use this to register your own planet generator
pub fn check_needs_generated_system<T: TGenerateChunkEvent + Event, K: Component>(
    mut commands: Commands,
    needs_generated_query: Query<(Entity, &ChunkNeedsGenerated)>,
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

#[derive(Debug, Clone, Copy)]
/// Send this event when a client requests a chunk
///
/// This will either generate a chunk & send it or send it if it's already loaded.
pub struct RequestChunkEvent {
    /// The client's id
    pub requester_id: u64,
    /// The structure's entity
    pub structure_entity: Entity,
    /// The chunk's coordinates on that structure
    pub chunk_coords: (usize, usize, usize),
}

#[derive(Debug, Clone, Copy)]
struct RequestChunkBouncer(RequestChunkEvent);

fn bounce_events(
    mut event_reader: EventReader<RequestChunkBouncer>,
    mut event_writer: EventWriter<RequestChunkEvent>,
) {
    for ev in event_reader.iter() {
        event_writer.send(ev.0);
    }
}

/// Performance hot spot
fn get_requested_chunk(
    mut event_reader: EventReader<RequestChunkEvent>,
    players: Query<&Location, With<Player>>,
    mut structure: Query<(&mut Structure, &Location), With<Planet>>,
    mut event_writer: EventWriter<RequestChunkBouncer>,
    mut server: ResMut<RenetServer>,
    mut commands: Commands,
) {
    let todo = Mutex::new(Some(Vec::new()));
    let serialized = Mutex::new(Some(Vec::new()));
    let bounced = Mutex::new(Some(Vec::new()));

    let non_empty_serializes = AtomicI32::new(0);
    let empty_serializes = AtomicI32::new(0);

    let timer = UtilsTimer::start();

    // No par_iter() for event readers, so first convert to vec then par_iter() it.
    event_reader
        .iter()
        .copied()
        .collect::<Vec<RequestChunkEvent>>()
        .par_iter()
        .for_each(|ev| {
            if let Ok((structure, loc)) = structure.get(ev.structure_entity) {
                let (cx, cy, cz) = ev.chunk_coords;

                let cpos = structure.chunk_relative_position(cx, cy, cz);

                let chunk_loc = *loc + cpos;

                // If no players are in range, do not send this chunk.
                if !players.iter().any(|player| {
                    player.relative_coords_to(&chunk_loc).abs().max_element() / CHUNK_DIMENSIONSF
                        < (RENDER_DISTANCE + 1) as f32
                }) {
                    return;
                }

                match structure.get_chunk_state(cx, cy, cz) {
                    ChunkState::Loaded => {
                        if let Some(chunk) = structure.chunk_from_chunk_coordinates(cx, cy, cz) {
                            let mut mutex = serialized.lock().expect("Failed to lock");

                            // let mut timer = UtilsTimer::start();
                            // let _ = cosmos_encoder::serialize(chunk);
                            // timer.log_duration("For bincode + compression:");
                            // timer.reset();
                            // let _ = bincode::serialize(chunk).unwrap();
                            // timer.log_duration("For just bincode:");

                            mutex.as_mut().unwrap().push((
                                ev.requester_id,
                                cosmos_encoder::serialize(&ServerReliableMessages::ChunkData {
                                    structure_entity: ev.structure_entity,
                                    serialized_chunk: cosmos_encoder::serialize(chunk),
                                }),
                            ));

                            non_empty_serializes.fetch_add(1, Ordering::SeqCst);
                        } else if structure.has_empty_chunk_at(cx, cy, cz) {
                            let mut mutex = serialized.lock().expect("Failed to lock");

                            mutex.as_mut().unwrap().push((
                                ev.requester_id,
                                cosmos_encoder::serialize(&ServerReliableMessages::EmptyChunk {
                                    structure_entity: ev.structure_entity,
                                    cx: cx as u32,
                                    cy: cy as u32,
                                    cz: cz as u32,
                                }),
                            ));

                            empty_serializes.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    ChunkState::Loading => bounced
                        .lock()
                        .expect("Failed to lock")
                        .as_mut()
                        .unwrap()
                        .push(RequestChunkBouncer(*ev)),
                    ChunkState::Unloaded => todo
                        .lock()
                        .expect("Failed to lock")
                        .as_mut()
                        .unwrap()
                        .push((ev.structure_entity, (cx, cy, cz), *ev)),
                    ChunkState::Invalid => {
                        eprintln!("Client requested invalid chunk @ {cx} {cy} {cz}");
                    }
                }
            }
        });

    let non_empty_serializes = non_empty_serializes.into_inner();
    let empty_serializes = empty_serializes.into_inner();

    if non_empty_serializes != 0 || empty_serializes != 0 {
        timer.log_duration(&format!("Time to serialize {non_empty_serializes} non-empty chunks & {empty_serializes} empty chunks:"));
    }

    for bounce in bounced.lock().expect("Failed to lock").take().unwrap() {
        event_writer.send(bounce);
    }

    for (client_id, serialized) in serialized.lock().expect("Failed to lock").take().unwrap() {
        server.send_message(client_id, NettyChannel::Reliable.id(), serialized);
    }

    for (entity, (cx, cy, cz), ev) in todo.lock().expect("Failed to lock").take().unwrap() {
        let Ok((mut structure, _)) = structure.get_mut(entity) else {
            continue;
        };

        mark_chunk_for_generation(&mut structure, &mut commands, cx, cy, cz, entity);

        event_writer.send(RequestChunkBouncer(ev));
    }
}

#[cfg(debug_assertions)]
const RENDER_DISTANCE: i32 = 2;
#[cfg(not(debug_assertions))]
const RENDER_DISTANCE: i32 = 3;

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

            let rd = RENDER_DISTANCE;

            let iterator = best_planet.chunk_iter(
                (px - rd, py - rd, pz - rd),
                (px + (rd), py + (rd), pz + (rd)),
                true,
            );

            let mut chunks = Vec::with_capacity(iterator.len());

            for chunk in iterator {
                if let ChunkIteratorResult::EmptyChunk {
                    position: (x, y, z),
                } = chunk
                {
                    if best_planet.get_chunk_state(x, y, z) == ChunkState::Unloaded {
                        chunks.push((x, y, z));
                    }
                }
            }

            for (x, y, z) in chunks {
                mark_chunk_for_generation(&mut best_planet, &mut commands, x, y, z, entity);
            }
        }
    }
}

fn mark_chunk_for_generation(
    structure: &mut Structure,
    commands: &mut Commands,
    cx: usize,
    cy: usize,
    cz: usize,
    structure_entity: Entity,
) {
    structure.mark_chunk_being_loaded(cx, cy, cz);

    let needs_generated_flag = commands
        .spawn((
            ChunkNeedsPopulated {
                chunk_coords: (cx, cy, cz),
                structure_entity,
            },
            NoSendEntity,
        ))
        .id();

    commands
        .entity(structure_entity)
        .add_child(needs_generated_flag);
}

fn unload_chunks_far_from_players(
    players: Query<&Location, With<Player>>,
    mut planets: Query<(&Location, &mut Structure, Entity, Option<&EntityId>), With<Planet>>,
    mut commands: Commands,
) {
    let mut potential_chunks = HashMap::<Entity, HashSet<(usize, usize, usize)>>::new();
    for (_, planet, entity, _) in planets.iter() {
        let mut set = HashSet::new();

        for chunk in planet.all_chunks_iter(false) {
            if let ChunkIteratorResult::FilledChunk {
                position: (cx, cy, cz),
                chunk: _,
            } = chunk
            {
                // Unloading chunks that are currently loading leads to bad things
                if planet.get_chunk_state(cx, cy, cz) == ChunkState::Loaded {
                    set.insert((cx, cy, cz));
                }
            }
        }

        potential_chunks.insert(entity, set);
    }

    for player in players.iter() {
        let mut best_planet = None;
        let mut best_dist = f32::INFINITY;
        for (location, structure, entity, entity_id) in planets.iter_mut() {
            let dist = location.distance_sqrd(player);
            if dist < best_dist {
                best_dist = dist;
                best_planet = Some((location, structure, entity, entity_id));
            }
        }

        if let Some((location, best_planet, entity, _)) = best_planet {
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

            let rd = RENDER_DISTANCE + 1;

            let iterator = best_planet.chunk_iter(
                (px - rd, py - rd, pz - rd),
                (px + (rd), py + (rd), pz + (rd)),
                true,
            );

            let set: &mut bevy::utils::hashbrown::HashSet<(usize, usize, usize)> = potential_chunks
                .get_mut(&entity)
                .expect("This was just added");

            for res in iterator {
                let chunk_position = match res {
                    ChunkIteratorResult::EmptyChunk { position } => position,
                    ChunkIteratorResult::FilledChunk { position, chunk: _ } => position,
                };

                set.remove(&chunk_position);
            }
        }
    }

    for (planet, set) in potential_chunks {
        if let Ok((location, mut structure, _, entity_id)) = planets.get_mut(planet) {
            let mut needs_id = false;

            let entity_id = if let Some(x) = entity_id {
                x.clone()
            } else {
                needs_id = true;
                EntityId::generate()
            };

            for (cx, cy, cz) in set {
                if let Some(chunk) = structure.unload_chunk_at(cx, cy, cz, &mut commands) {
                    let (cx, cy, cz) = (
                        chunk.structure_x(),
                        chunk.structure_y(),
                        chunk.structure_z(),
                    );

                    commands.spawn((
                        SaveChunk(chunk),
                        SaveFileIdentifier::as_child(
                            format!("{cx}_{cy}_{cz}"),
                            SaveFileIdentifier::new(
                                Some(location.sector()),
                                entity_id.clone(),
                                None,
                            ),
                        ),
                        NeedsSaved,
                        NeedsUnloaded,
                        NoSendEntity,
                    ));
                }
            }

            if needs_id {
                commands.entity(planet).insert((entity_id, NeedsSaved));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        (
            generate_chunks_near_players,
            unload_chunks_far_from_players,
            get_requested_chunk,
            bounce_events,
        )
            .chain()
            .in_set(OnUpdate(GameState::Playing)),
    )
    .add_event::<RequestChunkEvent>()
    .add_event::<RequestChunkBouncer>();
}
