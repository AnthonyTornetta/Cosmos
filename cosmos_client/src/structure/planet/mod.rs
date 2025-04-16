//! Handles client-related planet things

use bevy::{
    math::Quat,
    prelude::{
        App, Commands, Condition, Entity, EventWriter, GlobalTransform, IntoSystemConfigs, Mut, Query, Res, ResMut, Update, Vec3, With,
        in_state,
    },
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{
        NettyChannelClient, client::LocalPlayer, client_reliable_messages::ClientReliableMessages, cosmos_encoder,
        sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet,
    },
    physics::location::{Location, LocationPhysicsSet},
    state::GameState,
    structure::{
        ChunkState, Structure,
        chunk::{Chunk, ChunkUnloadEvent},
        coordinates::{UnboundChunkCoordinate, UnboundCoordinateType},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
    },
};

pub mod align_player;
pub mod biosphere;
pub mod client_planet_builder;
pub mod generation;
mod lods;
mod planet_skybox;
mod rotate_around_planet;

// #[cfg(debug_assertions)]
const RENDER_DISTANCE: UnboundCoordinateType = 2;
// #[cfg(not(debug_assertions))]
// const RENDER_DISTANCE: UnboundCoordinateType = 4;

fn find_player_planet_location<'a>(
    q_planets: &'a mut Query<(Entity, &Location, &mut Structure, &GlobalTransform), With<Planet>>,
    player_location: &Location,
) -> Option<(UnboundChunkCoordinate, Mut<'a, Structure>, Entity)> {
    let mut best_planet = None;
    let mut best_dist = f32::INFINITY;
    for (entity, location, structure, planet_g_trans) in q_planets.iter_mut() {
        let dist = location.distance_sqrd(player_location);
        if dist < best_dist {
            best_dist = dist;
            best_planet = Some((entity, location, structure, planet_g_trans));
        }
    }

    if let Some((entity, location, best_planet, planet_g_trans)) = best_planet {
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

fn load_planet_chunks(
    q_player_location: Query<&Location, With<LocalPlayer>>,
    mut q_planets: Query<(Entity, &Location, &mut Structure, &GlobalTransform), With<Planet>>,
    mapper: Res<NetworkMapping>,
    mut client: ResMut<RenetClient>,
) {
    let Ok(player_location) = q_player_location.get_single() else {
        return;
    };

    let Some((ub_chunk_coords, mut best_planet, planet_entity)) = find_player_planet_location(&mut q_planets, player_location) else {
        return;
    };

    let Some(server_entity) = mapper.server_from_client(&planet_entity) else {
        return;
    };

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

/// This system unloads chunks that are too far for a player to see.
///
/// Put systems that mess with chunks before this.
pub(crate) fn unload_chunks_far_from_players(
    q_player: Query<&Location, With<LocalPlayer>>,
    mut q_planets: Query<(&Location, &mut Structure, &GlobalTransform), With<Planet>>,
    mut event_writer: EventWriter<ChunkUnloadEvent>,
    mut commands: Commands,
) {
    let Ok(player) = q_player.get_single() else {
        return;
    };

    for (location, mut planet, planet_g_trans) in q_planets.iter_mut() {
        let player_relative_position: Vec3 = (*player - *location).into();
        let player_relative_position = Quat::from_affine3(&planet_g_trans.affine())
            .inverse()
            .mul_vec3(player_relative_position);

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

pub(super) fn register(app: &mut App) {
    align_player::register(app);
    biosphere::register(app);
    // lod::register(app);
    rotate_around_planet::register(app);
    lods::register(app);
    generation::register(app);
    planet_skybox::register(app);

    app.add_systems(
        Update,
        (load_planet_chunks, unload_chunks_far_from_players)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .after(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    );
}
