use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
    structure::{
        planet::{biosphere::BiosphereMarker, Planet},
        Structure,
    },
};

use crate::netty::sync::entities::RequestedEntityEvent;

fn on_request_planet(
    mut event_reader: EventReader<RequestedEntityEvent>,
    query: Query<(&Structure, &Planet, &BiosphereMarker)>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
        if let Ok((structure, planet, biosphere_marker)) = query.get(ev.entity) {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Planet {
                    entity: ev.entity,
                    width: structure.chunks_width() as u32,
                    height: structure.chunks_height() as u32,
                    length: structure.chunks_length() as u32,
                    planet: *planet,
                    biosphere: biosphere_marker.biosphere_name().to_owned(),
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(on_request_planet);
}
