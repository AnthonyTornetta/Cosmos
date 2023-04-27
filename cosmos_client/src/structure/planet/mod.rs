//! Handles client-related planet things

use bevy::prelude::{in_state, App, Entity, IntoSystemConfig, Query, Res, ResMut, Vec3, With};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_reliable_messages::ClientReliableMessages, cosmos_encoder, NettyChannel},
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

pub mod client_planet_builder;

fn load_planet_chunks(
    query: Query<&Location, With<LocalPlayer>>,
    mut planet: Query<(Entity, &Location, &mut Structure), With<Planet>>,
    mapper: Res<NetworkMapping>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(player) = query.get_single() {
        for (entity, location, mut best_planet) in planet.iter_mut() {
            if let Some(server_entity) = mapper.server_from_client(&entity) {
                println!("Player loc! {player} vs {location}");
                let player_relative_position: Vec3 = (*player - *location).into();
                let (px, py, pz) = best_planet.relative_coords_to_local_coords(
                    player_relative_position.x,
                    player_relative_position.y,
                    player_relative_position.z,
                );

                println!("Rel: {player_relative_position}");

                let (px, py, pz) = (
                    (px as f32 / CHUNK_DIMENSIONSF).floor() as i32,
                    (py as f32 / CHUNK_DIMENSIONSF).floor() as i32,
                    (pz as f32 / CHUNK_DIMENSIONSF).floor() as i32,
                );

                println!("P: {px} {py} {pz}");

                let mut chunks = vec![];

                for chunk in
                    best_planet.chunk_iter((px - 2, py - 2, pz - 2), (px + 2, py + 2, pz + 2), true)
                {
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

                    println!("REQUESTING CHUNK @ {x} {y} {z}!");

                    client.send_message(
                        NettyChannel::Reliable.id(),
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

pub(super) fn register(app: &mut App) {
    app.add_system(load_planet_chunks.run_if(in_state(GameState::Playing)));
}
