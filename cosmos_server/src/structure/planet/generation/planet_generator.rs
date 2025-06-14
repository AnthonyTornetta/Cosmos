//! Used to generate planets

use bevy::{
    ecs::event::Event,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_renet::renet::{ClientId, RenetServer};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::{EntityId, player::Player},
    netty::{
        NettyChannelServer, NoSendEntity, cosmos_encoder, server_reliable_messages::ServerReliableMessages,
        system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    state::GameState,
    structure::{
        ChunkState, Structure,
        chunk::{ChunkUnloadEvent, netty::SerializedBlockData},
        coordinates::{ChunkCoordinate, UnboundChunkCoordinate, UnboundCoordinateType},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
    },
};

use crate::{
    persistence::{
        SaveFileIdentifier,
        saving::{NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
    },
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
    parent_query: Query<&ChildOf>,
    correct_type_query: Query<(), With<K>>,
    mut event_writer: EventWriter<T>,
) {
    for (entity, chunk) in needs_generated_query.iter() {
        if let Ok(parent_entity) = parent_query.get(entity)
            && correct_type_query.contains(parent_entity.get()) {
                event_writer.write(T::new(chunk.coords, chunk.structure_entity));

                commands.entity(entity).despawn();
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
        event_writer.write(ev.0);
    }
}

/// Performance hot spot
fn get_requested_chunk(
    mut event_reader: EventReader<RequestChunkEvent>,
    // players: Query<&Location, With<Player>>,
    mut q_structure: Query<&mut Structure /*, &Location, &GlobalTransform*/, With<Planet>>,
    mut event_writer: EventWriter<RequestChunkBouncer>,
    mut server: ResMut<RenetServer>,
    mut commands: Commands,
) {
    let mut todo = Vec::new();
    let mut serialized = Vec::new();
    let mut bounced = Vec::new();

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
            if let Ok(structure /*loc, structure_g_trans*/) = q_structure.get(structure_entity) {
                // let cpos = structure.chunk_relative_position(chunk_coords);
                //
                // let structure_rot = Quat::from_affine3(&structure_g_trans.affine());
                // let chunk_rel_pos = structure_rot.mul_vec3(cpos);
                //
                // // TODO: If no players are in range, do not send this chunk.
                // //
                // // Also...
                // // TODO: We shold really ensure that the chunks a player is requesting are
                // // valid chunks for that player to request - even if it's valid for one player it
                // // may not be valid for the other.
                // if !players
                //     .iter()
                //     .map(|player_loc| Vec3::from(*player_loc - *loc))
                //     .any(|player_rel_pos| {
                //         (player_rel_pos - chunk_rel_pos).abs().max_element() / CHUNK_DIMENSIONSF < (RENDER_DISTANCE + 1) as f32
                //     })
                // {
                //     // TODO: We would have to send a denied message here, so the clients know to
                //     // re-request the chunk.
                //     return;
                // }

                match structure.get_chunk_state(chunk_coords) {
                    ChunkState::Loaded => {
                        if let Some(chunk) = structure.chunk_entity(chunk_coords) {
                            commands.entity(chunk).insert(ChunkNeedsSent { client_ids });
                        } else if structure.has_empty_chunk_at(chunk_coords) {
                            for client_id in client_ids {
                                serialized.push((
                                    client_id,
                                    cosmos_encoder::serialize(&ServerReliableMessages::EmptyChunk {
                                        structure_entity,
                                        coords: chunk_coords,
                                    }),
                                ));
                            }
                        }
                    }
                    ChunkState::Loading => {
                        for client_id in client_ids {
                            bounced.push(RequestChunkBouncer(RequestChunkEvent {
                                chunk_coords,
                                structure_entity,
                                requester_id: client_id,
                            }));
                        }
                    }
                    ChunkState::Unloaded => todo.push((structure_entity, chunk_coords, client_ids)),
                    ChunkState::Invalid => {
                        warn!("Client requested invalid chunk @ {}", chunk_coords);
                    }
                }
            }
        });

    for bounce in bounced {
        event_writer.write(bounce);
    }

    for (client_id, serialized) in serialized {
        server.send_message(client_id, NettyChannelServer::Reliable, serialized);
    }

    for (structure_entity, chunk_coords, client_ids) in todo {
        let Ok(mut structure) = q_structure.get_mut(structure_entity) else {
            continue;
        };

        mark_chunk_for_generation(&mut structure, &mut commands, chunk_coords, structure_entity);

        for client_id in client_ids {
            event_writer.write(RequestChunkBouncer(RequestChunkEvent {
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

fn find_player_planet_location<'a>(
    q_planets: &'a mut Query<(&Location, &mut Structure, Entity, &GlobalTransform), With<Planet>>,
    player_location: &Location,
) -> Option<(UnboundChunkCoordinate, Mut<'a, Structure>, Entity)> {
    let mut best_planet = None;
    let mut best_dist = f32::INFINITY;
    for (location, structure, entity, planet_g_trans) in q_planets.iter_mut() {
        let dist = location.distance_sqrd(player_location);
        if dist < best_dist {
            best_dist = dist;
            best_planet = Some((location, structure, entity, planet_g_trans));
        }
    }

    if let Some((location, best_planet, entity, planet_g_trans)) = best_planet {
        let player_relative_position: Vec3 = (*player_location - *location).into();
        let player_relative_position = Quat::from_affine3(&planet_g_trans.affine())
            .inverse()
            .mul_vec3(player_relative_position);

        let ub_coords =
            best_planet.relative_coords_to_local_coords(player_relative_position.x, player_relative_position.y, player_relative_position.z);

        let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ub_coords);
        Some((ub_chunk_coords, best_planet, entity))
    } else {
        None
    }
}

fn generate_chunks_near_players(
    players: Query<&Location, With<Player>>,
    mut q_planets: Query<(&Location, &mut Structure, Entity, &GlobalTransform), With<Planet>>,
    mut commands: Commands,
) {
    for player_location in players.iter() {
        if let Some((ub_chunk_coords, mut best_planet, planet_entity)) = find_player_planet_location(&mut q_planets, player_location) {
            let rd = RENDER_DISTANCE;

            let iterator = best_planet.chunk_iter(
                UnboundChunkCoordinate::new(ub_chunk_coords.x - rd, ub_chunk_coords.y - rd, ub_chunk_coords.z - rd),
                UnboundChunkCoordinate::new(ub_chunk_coords.x + rd, ub_chunk_coords.y + rd, ub_chunk_coords.z + rd),
                true,
            );

            let mut chunks = Vec::with_capacity(iterator.len());

            for chunk in iterator {
                if let ChunkIteratorResult::EmptyChunk { position: coords } = chunk
                    && best_planet.get_chunk_state(coords) == ChunkState::Unloaded {
                        chunks.push(coords);
                    }
            }

            for coords in chunks {
                mark_chunk_for_generation(&mut best_planet, &mut commands, coords, planet_entity);
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
    mut q_planets: Query<(&Location, &mut Structure, Entity, &GlobalTransform), With<Planet>>,
    q_entity_id: Query<&EntityId>,
    mut event_writer: EventWriter<ChunkUnloadEvent>,
    mut commands: Commands,
) {
    let mut chunks_to_unload = HashMap::<Entity, HashSet<ChunkCoordinate>>::new();
    for (_, planet, entity, _) in q_planets.iter() {
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

    for player_location in players.iter() {
        if let Some((ub_chunk_coords, best_planet, planet_entity)) = find_player_planet_location(&mut q_planets, player_location) {
            let rd = RENDER_DISTANCE + 1;

            let iterator = best_planet.chunk_iter(
                UnboundChunkCoordinate::new(ub_chunk_coords.x - rd, ub_chunk_coords.y - rd, ub_chunk_coords.z - rd),
                UnboundChunkCoordinate::new(ub_chunk_coords.x + rd, ub_chunk_coords.y + rd, ub_chunk_coords.z + rd),
                true,
            );

            let set = chunks_to_unload.get_mut(&planet_entity).expect("This was just added");

            for res in iterator {
                let chunk_position = match res {
                    ChunkIteratorResult::EmptyChunk { position } => position,
                    ChunkIteratorResult::FilledChunk { position, chunk: _ } => position,
                };

                set.remove(&chunk_position);
            }
        }
    }

    for (planet_entity, chunk_coords) in chunks_to_unload {
        if let Ok((location, mut structure, _, _)) = q_planets.get_mut(planet_entity) {
            let mut needs_id = false;

            let entity_id = q_entity_id.get(planet_entity);

            let entity_id = if let Ok(x) = entity_id {
                *x
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
                            SaveFileIdentifier::new(Some(location.sector()), entity_id, None),
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
                commands.entity(planet_entity).insert((entity_id, NeedsSaved));
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
