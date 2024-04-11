//! Handles client-related planet things

use bevy::prelude::{in_state, App, Commands, Entity, EventWriter, IntoSystemConfigs, Query, Res, ResMut, Update, Vec3, With};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, sync::mapping::NetworkMapping, NettyChannelClient},
    physics::location::Location,
    structure::{
        chunk::{Chunk, ChunkUnloadEvent},
        coordinates::{UnboundChunkCoordinate, UnboundCoordinateType},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
        ChunkState, Structure,
    },
};

use crate::{netty::flags::LocalPlayer, state::game_state::GameState};

pub mod align_player;
pub mod biosphere;
pub mod client_planet_builder;
pub mod generation;
mod lod;
mod lods;

#[cfg(debug_assertions)]
const RENDER_DISTANCE: UnboundCoordinateType = 2;
#[cfg(not(debug_assertions))]
const RENDER_DISTANCE: UnboundCoordinateType = 4;

fn load_planet_chunks(
    query: Query<&Location, With<LocalPlayer>>,
    mut planet: Query<(Entity, &Location, &mut Structure), With<Planet>>,
    mapper: Res<NetworkMapping>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(player) = query.get_single() {
        for (entity, location, mut best_planet) in planet.iter_mut() {
            if let Some(server_entity) = mapper.server_from_client(&entity) {
                let player_relative_position: Vec3 = (*player - *location).into();

                let coords = best_planet.relative_coords_to_local_coords(
                    player_relative_position.x,
                    player_relative_position.y,
                    player_relative_position.z,
                );

                let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(coords);

                let mut chunks = vec![];

                for chunk in best_planet.chunk_iter(
                    UnboundChunkCoordinate::new(
                        ub_chunk_coords.x - RENDER_DISTANCE,
                        ub_chunk_coords.y - RENDER_DISTANCE,
                        ub_chunk_coords.z - RENDER_DISTANCE,
                    ),
                    UnboundChunkCoordinate::new(
                        ub_chunk_coords.x + RENDER_DISTANCE,
                        ub_chunk_coords.y + RENDER_DISTANCE,
                        ub_chunk_coords.z + RENDER_DISTANCE,
                    ),
                    true,
                ) {
                    if let ChunkIteratorResult::EmptyChunk { position } = chunk {
                        if best_planet.get_chunk_state(position) == ChunkState::Unloaded {
                            chunks.push(position);
                        }
                    }
                }

                for coordinate in chunks {
                    best_planet.set_chunk(Chunk::new(coordinate));

                    client.send_message(
                        NettyChannelClient::Reliable,
                        cosmos_encoder::serialize(&ClientReliableMessages::SendSingleChunk {
                            structure_entity: server_entity,
                            chunk: coordinate,
                        }),
                    );
                }
            }
        }
    }
}

/// This system unloads chunks that are too far for a player to see.
///
/// Put systems that mess with chunks before this.
pub fn unload_chunks_far_from_players(
    player: Query<&Location, With<LocalPlayer>>,
    mut planets: Query<(&Location, &mut Structure), With<Planet>>,
    mut event_writer: EventWriter<ChunkUnloadEvent>,
    mut commands: Commands,
) {
    if let Ok(player) = player.get_single() {
        for (location, mut planet) in planets.iter_mut() {
            let player_relative_position: Vec3 = (*player - *location).into();
            let ub_coords =
                planet.relative_coords_to_local_coords(player_relative_position.x, player_relative_position.y, player_relative_position.z);

            let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ub_coords);

            let rd = RENDER_DISTANCE + 1;

            let mut chunks = Vec::new();

            for chunk in planet.all_chunks_iter(false) {
                if let ChunkIteratorResult::FilledChunk { position, chunk: _ } = chunk {
                    let (cx, cy, cz) = (
                        position.x as UnboundCoordinateType,
                        position.y as UnboundCoordinateType,
                        position.z as UnboundCoordinateType,
                    );

                    let (px, py, pz) = (ub_chunk_coords.x, ub_chunk_coords.y, ub_chunk_coords.z);

                    if !(cx >= px - rd && cx <= px + rd && cy >= py - rd && cy <= py + rd && cz >= pz - rd && cz <= pz + rd) {
                        chunks.push(position);
                    }
                }
            }

            let Structure::Dynamic(planet) = planet.as_mut() else {
                panic!("Invalid planet structure! It must be dynamic!");
            };

            for coordinate in chunks {
                planet.unload_chunk_at(coordinate, &mut commands, Some(&mut event_writer));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    align_player::register(app);
    biosphere::register(app);
    // lod::register(app);
    lods::register(app);
    generation::register(app);

    app.add_systems(
        Update,
        (load_planet_chunks, unload_chunks_far_from_players).run_if(in_state(GameState::Playing)),
    );
}
