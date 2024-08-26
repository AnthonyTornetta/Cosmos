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
use bevy_renet2::renet2::{ClientId, RenetServer};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::{
        cosmos_encoder, server_reliable_messages::ServerReliableMessages, system_sets::NetworkingSystemsSet, NettyChannelServer,
        NoSendEntity,
    },
    physics::location::Location,
    structure::{
        chunk::{netty::SerializedBlockData, ChunkUnloadEvent, CHUNK_DIMENSIONSF},
        coordinates::{ChunkCoordinate, UnboundChunkCoordinate, UnboundCoordinateType},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
        ChunkState, Structure,
    },
};

use crate::{
    persistence::{
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        EntityId, SaveFileIdentifier,
    },
    state::GameState,
    structure::{
        persistence::BlockDataNeedsSaved,
        planet::{
            biosphere::TGenerateChunkEvent,
            chunk::{ChunkNeedsSent, SaveChunk},
            persistence::ChunkNeedsPopulated,
        },
    },
};

#[derive(Component)]
/// This component will be in a planet's child entity if a chunk needs generated
///
/// This entity should be used as a flag, and is NOT the same as the chunk's entity
pub struct ChunkNeedsGenerated {
    /// The chunk's coordinates in the structure
    pub coords: ChunkCoordinate,
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
        if let Ok(parent_entity) = parent_query.get(entity) {
            if correct_type_query.contains(parent_entity.get()) {
                event_writer.send(T::new(chunk.coords, chunk.structure_entity));

                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Event)]
/// Send this event when a client requests a chunk
///
/// This will either generate a chunk & send it or send it if it's already loaded.
pub struct RequestChunkEvent {
    /// The client's id
    pub requester_id: ClientId,
    /// The structure's entity
    pub structure_entity: Entity,
    /// The chunk's coordinates on that structure
    pub chunk_coords: ChunkCoordinate,
}

#[derive(Debug, Clone, Copy, Event)]
struct RequestChunkBouncer(RequestChunkEvent);

fn bounce_events(mut event_reader: EventReader<RequestChunkBouncer>, mut event_writer: EventWriter<RequestChunkEvent>) {
    for ev in event_reader.read() {
        event_writer.send(ev.0);
    }
}

/// Performance hot spot
fn get_requested_chunk(
    mut event_reader: EventReader<RequestChunkEvent>,
    players: Query<&Location, With<Player>>,
    mut q_structure: Query<(&mut Structure, &Location), With<Planet>>,
    mut event_writer: EventWriter<RequestChunkBouncer>,
    mut server: ResMut<RenetServer>,
    mut commands: Commands,
) {
    let todo = Mutex::new(Some(Vec::new()));
    let serialized = Mutex::new(Some(Vec::new()));
    let bounced = Mutex::new(Some(Vec::new()));

    let non_empty_serializes = AtomicI32::new(0);
    let empty_serializes = AtomicI32::new(0);

    // let timer = UtilsTimer::start();

    let mut requests = HashMap::new();

    event_reader.read().for_each(|ev| {
        requests
            .entry((ev.structure_entity, ev.chunk_coords))
            .or_insert(vec![])
            .push(ev.requester_id);
    });

    // No par_iter() for event readers, so first convert to vec then par_iter() it.
    requests
        .into_iter()
        // .copied()
        // .collect::<Vec<RequestChunkEvent>>()
        // .par_iter()
        .for_each(|((structure_entity, chunk_coords), client_ids)| {
            if let Ok((structure, loc)) = q_structure.get(structure_entity) {
                let cpos = structure.chunk_relative_position(chunk_coords);

                let chunk_loc = *loc + cpos;

                // If no players are in range, do not send this chunk.
                if !players.iter().any(|player| {
                    player.relative_coords_to(&chunk_loc).abs().max_element() / CHUNK_DIMENSIONSF < (RENDER_DISTANCE + 1) as f32
                }) {
                    return;
                }

                match structure.get_chunk_state(chunk_coords) {
                    ChunkState::Loaded => {
                        if let Some(chunk) = structure.chunk_entity(chunk_coords) {
                            // let mut mutex = serialized.lock().expect("Failed to lock");

                            // let mut timer = UtilsTimer::start();
                            // let _ = cosmos_encoder::serialize(chunk);
                            // timer.log_duration("For bincode + compression:");
                            // timer.reset();
                            // let _ = bincode::serialize(chunk).unwrap();
                            // timer.log_duration("For just bincode:");

                            // mutex.as_mut().unwrap().push((
                            //     ev.requester_id,
                            //     cosmos_encoder::serialize(&ServerReliableMessages::ChunkData {
                            //         structure_entity: ev.structure_entity,
                            //         serialized_chunk: cosmos_encoder::serialize(chunk),
                            //         serialized_block_data: 0,
                            //     }),
                            // ));

                            commands.entity(chunk).insert(ChunkNeedsSent { client_ids });

                            non_empty_serializes.fetch_add(1, Ordering::SeqCst);
                        } else if structure.has_empty_chunk_at(chunk_coords) {
                            let mut mutex = serialized.lock().expect("Failed to lock");

                            let locked = mutex.as_mut().unwrap();

                            for client_id in client_ids {
                                locked.push((
                                    client_id,
                                    cosmos_encoder::serialize(&ServerReliableMessages::EmptyChunk {
                                        structure_entity,
                                        coords: chunk_coords,
                                    }),
                                ));
                            }

                            empty_serializes.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    ChunkState::Loading => {
                        let mut locked = bounced.lock().expect("Failed to lock");
                        let locked = locked.as_mut().unwrap();
                        for client_id in client_ids {
                            locked.push(RequestChunkBouncer(RequestChunkEvent {
                                chunk_coords,
                                structure_entity,
                                requester_id: client_id,
                            }));
                        }
                    }
                    ChunkState::Unloaded => {
                        todo.lock()
                            .expect("Failed to lock")
                            .as_mut()
                            .unwrap()
                            .push((structure_entity, chunk_coords, client_ids))
                    }
                    ChunkState::Invalid => {
                        warn!("Client requested invalid chunk @ {}", chunk_coords);
                    }
                }
            }
        });

    // let non_empty_serializes = non_empty_serializes.into_inner();
    // let empty_serializes = empty_serializes.into_inner();

    // if non_empty_serializes != 0 || empty_serializes != 0 {
    //     timer.log_duration(&format!(
    //         "Time to serialize {non_empty_serializes} non-empty chunks & {empty_serializes} empty chunks:"
    //     ));
    // }

    for bounce in bounced.lock().expect("Failed to lock").take().unwrap() {
        event_writer.send(bounce);
    }

    for (client_id, serialized) in serialized.lock().expect("Failed to lock").take().unwrap() {
        server.send_message(client_id, NettyChannelServer::Reliable, serialized);
    }

    for (structure_entity, chunk_coords, client_ids) in todo.lock().expect("Failed to lock").take().unwrap() {
        let Ok((mut structure, _)) = q_structure.get_mut(structure_entity) else {
            continue;
        };

        mark_chunk_for_generation(&mut structure, &mut commands, chunk_coords, structure_entity);

        for client_id in client_ids {
            event_writer.send(RequestChunkBouncer(RequestChunkEvent {
                chunk_coords,
                structure_entity,
                requester_id: client_id,
            }));
        }
    }
}

// #[cfg(debug_assertions)]
const RENDER_DISTANCE: UnboundCoordinateType = 2;
// #[cfg(not(debug_assertions))]
// const RENDER_DISTANCE: UnboundCoordinateType = 4;

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
            let ub_coords = best_planet.relative_coords_to_local_coords(
                player_relative_position.x,
                player_relative_position.y,
                player_relative_position.z,
            );

            let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ub_coords);

            let rd = RENDER_DISTANCE;

            let iterator = best_planet.chunk_iter(
                UnboundChunkCoordinate::new(ub_chunk_coords.x - rd, ub_chunk_coords.y - rd, ub_chunk_coords.z - rd),
                UnboundChunkCoordinate::new(ub_chunk_coords.x + rd, ub_chunk_coords.y + rd, ub_chunk_coords.z + rd),
                true,
            );

            let mut chunks = Vec::with_capacity(iterator.len());

            for chunk in iterator {
                if let ChunkIteratorResult::EmptyChunk { position: coords } = chunk {
                    if best_planet.get_chunk_state(coords) == ChunkState::Unloaded {
                        chunks.push(coords);
                    }
                }
            }

            for coords in chunks {
                mark_chunk_for_generation(&mut best_planet, &mut commands, coords, entity);
            }
        }
    }
}

fn mark_chunk_for_generation(structure: &mut Structure, commands: &mut Commands, coords: ChunkCoordinate, structure_entity: Entity) {
    let Structure::Dynamic(planet) = structure else {
        panic!("A planet must be dynamic!");
    };
    planet.mark_chunk_being_loaded(coords);

    let needs_generated_flag = commands
        .spawn((
            ChunkNeedsPopulated {
                chunk_coords: coords,
                structure_entity,
            },
            NoSendEntity,
        ))
        .id();

    commands.entity(structure_entity).add_child(needs_generated_flag);
}

fn unload_chunks_far_from_players(
    players: Query<&Location, With<Player>>,
    mut planets: Query<(&Location, &mut Structure, Entity, Option<&EntityId>), With<Planet>>,
    mut event_writer: EventWriter<ChunkUnloadEvent>,
    mut commands: Commands,
) {
    let mut chunks_to_unload = HashMap::<Entity, HashSet<ChunkCoordinate>>::new();
    for (_, planet, entity, _) in planets.iter() {
        let mut set = HashSet::new();

        for chunk in planet.all_chunks_iter(false) {
            if let ChunkIteratorResult::FilledChunk {
                position: coords,
                chunk: _,
            } = chunk
            {
                // Unloading chunks that are currently loading leads to bad things
                if planet.get_chunk_state(coords) == ChunkState::Loaded {
                    set.insert(coords);
                }
            }
        }

        chunks_to_unload.insert(entity, set);
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
            let ub_coords = best_planet.relative_coords_to_local_coords(
                player_relative_position.x,
                player_relative_position.y,
                player_relative_position.z,
            );

            let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ub_coords);

            let rd = RENDER_DISTANCE + 1;

            let iterator = best_planet.chunk_iter(
                UnboundChunkCoordinate::new(ub_chunk_coords.x - rd, ub_chunk_coords.y - rd, ub_chunk_coords.z - rd),
                UnboundChunkCoordinate::new(ub_chunk_coords.x + rd, ub_chunk_coords.y + rd, ub_chunk_coords.z + rd),
                true,
            );

            let set = chunks_to_unload.get_mut(&entity).expect("This was just added");

            for res in iterator {
                let chunk_position = match res {
                    ChunkIteratorResult::EmptyChunk { position } => position,
                    ChunkIteratorResult::FilledChunk { position, chunk: _ } => position,
                };

                set.remove(&chunk_position);
            }
        }
    }

    for (planet, chunk_coords) in chunks_to_unload {
        if let Ok((location, mut structure, _, entity_id)) = planets.get_mut(planet) {
            let mut needs_id = false;

            let entity_id = if let Some(x) = entity_id {
                x.clone()
            } else {
                needs_id = true;
                EntityId::generate()
            };

            for coords in chunk_coords {
                let Structure::Dynamic(planet) = structure.as_mut() else {
                    panic!("A planet must be dynamic!");
                };

                if let Some(chunk) = planet.unload_chunk_at(coords, &mut commands, Some(&mut event_writer)) {
                    let (cx, cy, cz) = (coords.x, coords.y, coords.z);

                    let mut ecmds = commands.spawn((
                        SaveFileIdentifier::as_child(
                            format!("{cx}_{cy}_{cz}"),
                            SaveFileIdentifier::new(Some(location.sector()), entity_id.clone(), None),
                        ),
                        NeedsSaved,
                        NeedsDespawned,
                        NoSendEntity,
                    ));

                    if !chunk.all_block_data_entities().is_empty() {
                        ecmds.insert(SerializedBlockData::new(coords));
                    }

                    let save_ent = ecmds.id();

                    for (_, &entity) in chunk.all_block_data_entities() {
                        // Block data saving relies on the block data being a child of the thing being saved,
                        // so make that hold true here
                        commands.entity(entity).insert(BlockDataNeedsSaved).set_parent(save_ent);
                    }

                    commands.entity(save_ent).insert(SaveChunk(chunk));
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
        Update,
        (generate_chunks_near_players, get_requested_chunk, bounce_events)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        SAVING_SCHEDULE,
        unload_chunks_far_from_players
            .before(SavingSystemSet::BeginSaving)
            .run_if(in_state(GameState::Playing)),
    )
    .add_event::<RequestChunkEvent>()
    .add_event::<RequestChunkBouncer>();
}
