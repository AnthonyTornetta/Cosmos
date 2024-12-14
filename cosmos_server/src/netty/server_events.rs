//! Handles client connecting and disconnecting

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet2::renet2::transport::NetcodeServerTransport;
use bevy_renet2::renet2::{ClientId, RenetServer, ServerEvent};
use cosmos_core::ecs::NeedsDespawned;
use cosmos_core::entities::player::render_distance::RenderDistance;
use cosmos_core::netty::netty_rigidbody::NettyRigidBodyLocation;
use cosmos_core::netty::server::ServerLobby;
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::netty::sync::server_entity_syncing::RequestedEntityEvent;
use cosmos_core::netty::{cosmos_encoder, NettyChannelServer};
use cosmos_core::physics::location::Location;
use cosmos_core::{entities::player::Player, netty::netty_rigidbody::NettyRigidBody};
use renet2_visualizer::RenetServerVisualizer;

use crate::entities::player::persistence::LoadPlayer;
use crate::netty::network_helpers::ClientTicks;
use crate::persistence::saving::NeedsSaved;

#[derive(Event, Debug)]
/// Sent whenever a player just connected
pub struct PlayerConnectedEvent {
    /// The player's entity
    pub player_entity: Entity,
    /// Player's client id
    pub client_id: ClientId,
}

pub(super) fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    q_players: Query<(Entity, &Player, &Transform, &Location, &Velocity, &RenderDistance)>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    mut requested_entity: EventWriter<RequestedEntityEvent>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client {client_id} connected");
                visualizer.add_client(client_id);

                for (entity, player, transform, location, velocity, render_distance) in q_players.iter() {
                    let body = NettyRigidBody::new(Some(*velocity), transform.rotation, NettyRigidBodyLocation::Absolute(*location));

                    let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
                        entity,
                        id: player.id(),
                        body,
                        name: player.name().to_owned(),
                        render_distance: Some(*render_distance),
                    });

                    server.send_message(client_id, NettyChannelServer::Reliable, msg);
                    requested_entity.send(RequestedEntityEvent { client_id, entity });
                }

                let Some(user_data) = transport.user_data(client_id) else {
                    warn!("Unable to get user data!");
                    continue;
                };
                let Ok(name) = bincode::deserialize::<String>(user_data.as_slice()) else {
                    warn!("Unable to deserialize name!");
                    continue;
                };

                commands.spawn(LoadPlayer { name, client_id });
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Client {client_id} disconnected: {reason}");
                visualizer.remove_client(*client_id);
                client_ticks.ticks.remove(client_id);

                if let Some(player_entity) = lobby.remove_player(*client_id) {
                    commands.entity(player_entity).insert((NeedsSaved, NeedsDespawned));
                }

                let message = cosmos_encoder::serialize(&ServerReliableMessages::PlayerRemove { id: *client_id });

                server.broadcast_message(NettyChannelServer::Reliable, message);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<PlayerConnectedEvent>();
}
