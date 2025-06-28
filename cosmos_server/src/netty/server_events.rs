//! Handles client connecting and disconnecting

use bevy::prelude::*;
use bevy_renet::renet::{ClientId, RenetServer, ServerEvent};
use bevy_renet::steam::steamworks::SteamId;
use cosmos_core::ecs::NeedsDespawned;
use cosmos_core::entities::player::Player;
use cosmos_core::netty::server::ServerLobby;
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::netty::{NettyChannelServer, cosmos_encoder};
use renet_steam::SteamServerTransport;
use renet_visualizer::RenetServerVisualizer;

use crate::entities::player::persistence::LoadPlayer;
use crate::init::init_server::ServerSteamClient;
use crate::netty::network_helpers::ClientTicks;
use crate::persistence::saving::NeedsSaved;

// use super::auth::AuthenticationServer;

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
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    q_players: Query<&Player>,
    steam_client: Res<ServerSteamClient>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client {client_id} connected");

                // let Some(user_data) = transport.(client_id) else {
                //     warn!("Unable to get user data - rejecting connection!");
                //     server.disconnect(client_id);
                //     continue;
                // };

                // match auth_server.as_ref() {
                //     AuthenticationServer::Steam(s) => {
                //         s.begin_authentication_session(user, ticket);
                //     }
                //     AuthenticationServer::None => {}
                // }
                //
                let name = steam_client.client().friends().get_friend(SteamId::from_raw(client_id)).name();

                // let Ok(name) = cosmos_encoder::deserialize_uncompressed::<String>(user_data.as_slice()) else {
                //     warn!("Unable to deserialize name - rejecting connection!");
                //     server.disconnect(client_id);
                //     continue;
                // };

                if q_players.iter().any(|x| x.name() == name) {
                    warn!("Duplicate name - rejecting connection!");
                    server.disconnect(client_id);
                    continue;
                }

                visualizer.add_client(client_id);
                commands.spawn(LoadPlayer { name, client_id });
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Client {client_id} disconnected: {reason}");
                visualizer.remove_client(*client_id);
                client_ticks.ticks.remove(client_id);

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
    app.add_event::<PlayerConnectedEvent>();
}
