use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{NettyChannelServer, cosmos_encoder, sync::server_entity_syncing::RequestedEntityEvent, system_sets::NetworkingSystemsSet},
    structure::{
        Structure,
        asteroid::{Asteroid, asteroid_netty::AsteroidServerMessages},
    },
};

fn on_request_asteroid(
    mut event_reader: EventReader<RequestedEntityEvent>,
    query: Query<(&Structure, &Asteroid)>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, asteroid)) = query.get(ev.entity) {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Asteroid,
                cosmos_encoder::serialize(&AsteroidServerMessages::Asteroid {
                    entity: ev.entity,
                    dimensions: structure.chunk_dimensions(),
                    temperature: asteroid.temperature(),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_request_asteroid.in_set(NetworkingSystemsSet::SyncComponents));
}
