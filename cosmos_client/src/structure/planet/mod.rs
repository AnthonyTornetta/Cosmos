//! Handles client-related planet things

use bevy::prelude::{
    in_state, App, Commands, Entity, IntoSystemConfig, OnUpdate, Query, Res, ResMut, Vec3, With,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannelClient},
    physics::location::Location,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONSF},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
        ChunkState, Structure,
    },
};

use crate::{
    netty::{flags::LocalPlayer, mapping::NetworkMapping},
    state::game_state::GameState,
};

pub mod align_player;
pub mod biosphere;
pub mod client_planet_builder;

#[cfg(debug_assertions)]
const RENDER_DISTANCE: i32 = 2;
#[cfg(not(debug_assertions))]
const RENDER_DISTANCE: i32 = 3;

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

                let rd = RENDER_DISTANCE;

                for chunk in best_planet.chunk_iter(
                    (px - rd, py - rd, pz - rd),
                    (px + rd, py + rd, pz + rd),
                    true,
                ) {
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
                    best_planet.set_chunk(Chunk::new(x, y, z));

                    client.send_message(
                        NettyChannelClient::Reliable,
                        cosmos_encoder::serialize(&ClientReliableMessages::SendSingleChunk {
                            structure_entity: server_entity,
                            chunk: (x as u32, y as u32, z as u32),
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
    mut commands: Commands,
) {
    if let Ok(player) = player.get_single() {
        for (location, mut planet) in planets.iter_mut() {
            let player_relative_position: Vec3 = (*player - *location).into();
            let (px, py, pz) = planet.relative_coords_to_local_coords(
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

            let mut chunks = Vec::new();

            for chunk in planet.all_chunks_iter(false) {
                if let ChunkIteratorResult::FilledChunk {
                    position: (cx, cy, cz),
                    chunk: _,
                } = chunk
                {
                    if !(cx as i32 >= px - rd
                        && cx as i32 <= px + rd
                        && cy as i32 >= py - rd
                        && cy as i32 <= py + rd
                        && cz as i32 >= pz - rd
                        && cz as i32 <= pz + rd)
                    {
                        chunks.push((cx, cy, cz));
                    }
                }
            }

            for (cx, cy, cz) in chunks {
                planet.unload_chunk_at(cx, cy, cz, &mut commands);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    align_player::register(app);
    client_planet_builder::register(app);
    biosphere::register(app);

    app.add_system(load_planet_chunks.in_set(OnUpdate(GameState::Playing)))
        .add_system(unload_chunks_far_from_players.run_if(in_state(GameState::Playing)));
}
