//! Handles the syncing of entity's rigidbodies + velocities

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::{render_distance::RenderDistance, Player},
    netty::{
        cosmos_encoder, netty_rigidbody::NettyRigidBody,
        server_unreliable_messages::ServerUnreliableMessages, NettyChannel, NoSendEntity,
    },
    physics::location::{Location, SECTOR_DIMENSIONS},
};

use crate::netty::network_helpers::NetworkTick;

/// Sends bodies to players only if it's within their render distance.
fn send_bodies(
    players: &Query<(&Player, &RenderDistance, &Location)>,
    bodies: &[(Entity, NettyRigidBody)],
    server: &mut RenetServer,
    tick: &NetworkTick,
) {
    for (player, rd, loc) in players.iter() {
        let rd =
            rd.sector_range as f32 * SECTOR_DIMENSIONS * rd.sector_range as f32 * SECTOR_DIMENSIONS;

        let players_bodies: Vec<(Entity, NettyRigidBody)> = bodies
            .iter()
            .filter(|(_, rb)| rb.location.distance_sqrd(loc) < rd)
            .copied()
            .collect();

        if !players_bodies.is_empty() {
            let sync_message = ServerUnreliableMessages::BulkBodies {
                time_stamp: tick.0,
                bodies: players_bodies,
            };

            let message = cosmos_encoder::serialize(&sync_message);

            server.send_message(player.id(), NettyChannel::Unreliable.id(), message.clone());
        }
    }
}

/// Only sends entities that changed locations
fn server_sync_bodies(
    mut server: ResMut<RenetServer>,
    mut tick: ResMut<NetworkTick>,
    entities: Query<(Entity, &Transform, &Location, &Velocity), Without<NoSendEntity>>,
    players: Query<(&Player, &RenderDistance, &Location)>,
) {
    tick.0 += 1;

    let mut bodies = Vec::new();

    for (entity, transform, location, velocity) in entities.iter() {
        bodies.push((
            entity,
            NettyRigidBody::new(velocity, transform.rotation, *location),
        ));

        // The packet size can only be so big, so limit syncing to 20 per packet
        if bodies.len() > 20 {
            send_bodies(&players, &bodies, &mut server, &tick);
            bodies = Vec::new();
        }
    }

    if !bodies.is_empty() {
        send_bodies(&players, &bodies, &mut server, &tick);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(server_sync_bodies);
}
