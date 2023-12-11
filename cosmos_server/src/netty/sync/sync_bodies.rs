//! Handles the syncing of entity's rigidbodies + velocities

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::{render_distance::RenderDistance, Player},
    netty::{
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages,
        system_sets::NetworkingSystemsSet,
        NettyChannelServer, NoSendEntity,
    },
    persistence::LoadingDistance,
    physics::location::{add_previous_location, Location},
};

use crate::netty::network_helpers::NetworkTick;

use super::entities::RequestedEntityEvent;

/// Sends bodies to players only if it's within their render distance.
fn send_bodies(
    players: &Query<(&Player, &RenderDistance, &Location)>,
    bodies: &[(Entity, NettyRigidBody, Location, LoadingDistance)],
    server: &mut RenetServer,
    tick: &NetworkTick,
) {
    for (player, _, loc) in players.iter() {
        let players_bodies: Vec<(Entity, NettyRigidBody)> = bodies
            .iter()
            .filter(|(_, _, location, loading_distance)| {
                location.relative_coords_to(loc).abs().max_element() < loading_distance.load_block_distance()
            })
            .map(|(ent, net_rb, _, _)| (*ent, *net_rb))
            .collect();

        if !players_bodies.is_empty() {
            let sync_message = ServerUnreliableMessages::BulkBodies {
                time_stamp: tick.0,
                bodies: players_bodies,
            };

            let message = cosmos_encoder::serialize(&sync_message);

            server.send_message(player.id(), NettyChannelServer::Unreliable, message.clone());
        }
    }
}

fn server_sync_bodies(
    mut server: ResMut<RenetServer>,
    mut tick: ResMut<NetworkTick>,
    location_query: Query<&Location>,
    entities: Query<(Entity, &Transform, &Location, &Velocity, &LoadingDistance, Option<&Parent>), Without<NoSendEntity>>,
    players: Query<(&Player, &RenderDistance, &Location)>,
) {
    tick.0 += 1;

    let mut bodies = Vec::new();

    for (entity, transform, location, velocity, unload_distance, parent) in entities.iter() {
        bodies.push((
            entity,
            NettyRigidBody::new(
                velocity,
                transform.rotation,
                match parent.map(|p| p.get()) {
                    Some(parent_entity) => NettyRigidBodyLocation::Relative(
                        (*location - location_query.get(parent_entity).copied().unwrap_or(Location::default())).absolute_coords_f32(),
                        parent_entity,
                    ),
                    None => NettyRigidBodyLocation::Absolute(*location),
                },
            ),
            *location,
            *unload_distance,
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

fn pinger(mut server: ResMut<RenetServer>, mut event_reader: EventReader<RequestedEntityEvent>, mut commands: Commands) {
    for ev in event_reader.read() {
        if commands.get_entity(ev.entity).is_some() {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::RequestedEntityReceived(ev.entity)),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        // This really needs to run immediately after `add_previous_location` to make sure nothing causes any desync
        // in location + transform, but for now it's fine.
        (
            server_sync_bodies
                .after(add_previous_location)
                .before(NetworkingSystemsSet::PreReceiveMessages),
            pinger,
        ),
    );
}
