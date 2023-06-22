use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        cosmos_encoder, netty_rigidbody::NettyRigidBody,
        server_reliable_messages::ServerReliableMessages, NettyChannelServer,
    },
    physics::location::Location,
    structure::{loading::ChunksNeedLoaded, ship::Ship, Structure},
};

use crate::netty::sync::entities::RequestedEntityEvent;

fn on_request_ship(
    mut event_reader: EventReader<RequestedEntityEvent>,
    query: Query<(&Structure, &Transform, &Location, &Velocity), With<Ship>>,
    mut server: ResMut<RenetServer>,
) {
    for ev in event_reader.iter() {
        if let Ok((structure, transform, location, velocity)) = query.get(ev.entity) {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::Ship {
                    entity: ev.entity,
                    body: NettyRigidBody::new(velocity, transform.rotation, *location),
                    width: structure.chunks_width() as u32,
                    height: structure.chunks_height() as u32,
                    length: structure.chunks_length() as u32,
                    chunks_needed: ChunksNeedLoaded {
                        amount_needed: structure.all_chunks_iter(false).len(),
                    },
                }),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(on_request_ship);
}
