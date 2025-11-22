//! Handles client connecting and disconnecting

use bevy::prelude::*;
use bevy_renet::renet::{ClientId, RenetServer};
use cosmos_core::ecs::NeedsDespawned;
use cosmos_core::entities::player::Player;
use cosmos_core::netty::client_preconnect_messages::ClientPreconnectMessages;
use cosmos_core::netty::server::ServerLobby;
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::netty::{NettyChannelClient, NettyChannelServer, cosmos_encoder};
use renet::ServerEvent;
use renet_visualizer::RenetServerVisualizer;

use crate::entities::player::persistence::LoadPlayer;
use crate::netty::network_helpers::ClientTicks;
use crate::persistence::saving::NeedsSaved;

// use super::auth::AuthenticationServer;

#[derive(Message, Debug)]
/// Sent whenever a player just connected
pub struct PlayerConnectedMessage {
    /// The player's entity
    pub player_entity: Entity,
    /// Player's client id
    pub client_id: ClientId,
}

#[derive(Component, Reflect)]
pub(crate) struct PreconnectedPlayer {
    client_id: ClientId,
    name: Option<String>,
}

impl PreconnectedPlayer {
    pub fn ready(&self) -> bool {
        self.name.is_some()
    }
}

fn handle_pre_connect_messages(
    mut server: ResMut<RenetServer>,
    q_players: Query<&Player>,
    mut q_pre_connections: Query<(Entity, &mut PreconnectedPlayer)>,
    mut commands: Commands,
) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::PreConnect) {
            let Ok(message) = cosmos_encoder::deserialize::<ClientPreconnectMessages>(&message) else {
                error!("Failed to deserialize preconnect message!");
                continue;
            };

            info!("{}", q_pre_connections.iter().count());

            let Some((ent, _)) = q_pre_connections.iter().find(|(_, p)| p.client_id == client_id) else {
                error!("Rejecting preconnection message from {client_id} - they are not in the list.");
                continue;
            };

            match message {
                ClientPreconnectMessages::Init { name } => {
                    if q_players.iter().any(|x| x.name() == name)
                        || q_pre_connections
                            .iter()
                            .any(|(_, pre)| pre.name.as_ref().map(|n| n == &name).unwrap_or(false))
                    {
                        warn!("Duplicate name ({name}) - rejecting connection!");
                        server.disconnect(client_id);
                        commands.entity(ent).despawn();
                        continue;
                    }

                    // can't do this above - borrow checker
                    if let Some((_, mut precon_player)) = q_pre_connections.iter_mut().find(|(_, p)| p.client_id == client_id) {
                        precon_player.name = Some(name);
                    }
                }
            }
        }
    }
}

fn on_change_preconnect_player(
    mut commands: Commands,
    q_pre_connections: Query<(Entity, &PreconnectedPlayer), Changed<PreconnectedPlayer>>,
) {
    for (ent, precon_player) in q_pre_connections.iter() {
        if !precon_player.ready() {
            continue;
        }

        commands.entity(ent).despawn();
        info!("Pre-connect done for {} - spawning player now!", precon_player.client_id);
        commands.spawn(LoadPlayer {
            name: precon_player.name.clone().expect("Missing name but was validated"),
            client_id: precon_player.client_id,
        });
    }
}

pub(super) fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut server_events: MessageReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    q_pre_connections: Query<(Entity, &PreconnectedPlayer)>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client {client_id} pre-connected");

                if q_pre_connections.iter().any(|(_, x)| x.client_id == client_id) {
                    warn!("Duplicate client id - rejecting connection!");
                    server.disconnect(client_id);
                }

                commands.spawn((
                    Name::new(format!("Preconnect {client_id}")),
                    PreconnectedPlayer { client_id, name: None },
                ));

                visualizer.add_client(client_id);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Client {client_id} disconnected: {reason}");
                visualizer.remove_client(*client_id);
                client_ticks.ticks.remove(client_id);

                if let Some((ent, _)) = q_pre_connections.iter().find(|(_, x)| x.client_id == *client_id) {
                    commands.entity(ent).despawn();
                }

                if let Some(player_entity) = lobby.remove_player(*client_id)
                    && let Ok(mut ecmds) = commands.get_entity(player_entity)
                {
                    ecmds.insert((NeedsSaved, NeedsDespawned));
                }

                let message = cosmos_encoder::serialize(&ServerReliableMessages::PlayerRemove { id: *client_id });

                server.broadcast_message(NettyChannelServer::Reliable, message);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_message::<PlayerConnectedMessage>();

    app.add_systems(
        FixedUpdate,
        (handle_pre_connect_messages.after(handle_server_events), on_change_preconnect_player)
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );
}
