//! Server registry syncing

use bevy::prelude::*;
use cosmos_core::{
    netty::{
        NettyChannelClient, client_registry::RegistrySyncing, cosmos_encoder, server::ServerLobby, sync::server_syncing::ReadyForSyncing,
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};
use renet::{ClientId, RenetServer};

#[derive(Debug, Event)]
/// This event is sent when the client has received every registry from the server.
///
/// This will be sent in their initial connecting phase, and anything that relies on a registry
/// must be sent after this is received.
pub struct ClientFinishedReceivingRegistriesEvent(pub ClientId);

fn listen_for_done_syncing(
    mut server: ResMut<RenetServer>,
    mut evw_finished_receiving_registries: EventWriter<ClientFinishedReceivingRegistriesEvent>,
    lobby: Res<ServerLobby>,
    mut commands: Commands,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Registry) {
            let Some(player_ent) = lobby.player_from_id(client_id) else {
                continue;
            };
            let Ok(msg) = cosmos_encoder::deserialize::<RegistrySyncing>(&message) else {
                warn!("Bad deserialization");
                continue;
            };

            info!("Got registry message from client {client_id}");

            commands.entity(player_ent).insert(ReadyForSyncing);

            match msg {
                RegistrySyncing::FinishedReceivingRegistries => {
                    evw_finished_receiving_registries.write(ClientFinishedReceivingRegistriesEvent(client_id));
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        listen_for_done_syncing
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    )
    .add_event::<ClientFinishedReceivingRegistriesEvent>();
}
