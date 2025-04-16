use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages,
        sync::server_entity_syncing::RequestedEntityEvent, system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    structure::{
        Structure,
        planet::{Planet, biosphere::BiosphereMarker},
    },
};

fn on_request_planet(
    mut event_reader: EventReader<RequestedEntityEvent>,
    query: Query<(&Structure, &Planet, &Location, &BiosphereMarker)>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, planet, location, biosphere_marker)) = query.get(ev.entity) {
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
                    biosphere: biosphere_marker.biosphere_name().to_owned(),
                    location: *location,
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_request_planet.in_set(NetworkingSystemsSet::SyncComponents));
}
