use bevy::prelude::*;
use bevy_renet::RenetServer;
use cosmos_core::{
    netty::{
        NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages,
        sync::server_entity_syncing::RequestedEntityMessage, system_sets::NetworkingSystemsSet,
    },
    structure::{
        Structure,
        planet::{Planet, biosphere::BiosphereMarker, generation::terrain_generation::PlanetTerrainSeed},
    },
};

fn on_request_planet(
    mut event_reader: MessageReader<RequestedEntityMessage>,
    query: Query<(&Structure, &Planet, &PlanetTerrainSeed, &BiosphereMarker)>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, planet, terrain_seed, biosphere_marker)) = query.get(ev.entity) {
            let Structure::Dynamic(dynamic_planet) = structure else {
                panic!("Planet must be dynamic!");
            };

            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Planet {
                    entity: ev.entity,
                    dimensions: dynamic_planet.chunk_dimensions(),
                    planet: *planet,
                    terrain_seed: terrain_seed.value(),
                    biosphere: biosphere_marker.biosphere_name().to_owned(),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_request_planet.in_set(NetworkingSystemsSet::SyncComponents));
}
