use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::netty::{
    netty_rigidbody::NettyRigidBody, server_unreliable_messages::ServerUnreliableMessages,
    NettyChannel,
};

use crate::netty::network_helpers::NetworkTick;

pub fn server_sync_bodies(
    mut server: ResMut<RenetServer>,
    mut tick: ResMut<NetworkTick>,
    entities: Query<(Entity, &Transform, &Velocity)>,
) {
    tick.0 += 1;

    let mut bodies = Vec::new();

    for (entity, transform, velocity) in entities.iter() {
        bodies.push((entity, NettyRigidBody::new(velocity, transform)));

        // The packet size can only be so big, so limit syncing to 20 per packet
        if bodies.len() > 20 {
            let sync_message = ServerUnreliableMessages::BulkBodies {
                time_stamp: tick.0,
                bodies,
            };
            let message = bincode::serialize(&sync_message).unwrap();

            server.broadcast_message(NettyChannel::Unreliable.id(), message);

            bodies = Vec::new();
        }
    }

    if !bodies.is_empty() {
        let sync_message = ServerUnreliableMessages::BulkBodies {
            time_stamp: tick.0,
            bodies,
        };
        let message = bincode::serialize(&sync_message).unwrap();

        server.broadcast_message(NettyChannel::Unreliable.id(), message);
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_sync_bodies);
}
