//! Contains all the connection functionality from client -> server
//!
//! This does not add them to the bevy systems by default, and they must be manually added when needed.

use std::net::SocketAddr;

use bevy::prelude::*;
use bevy_renet::{renet::RenetClient, steam::steamworks::SteamId};
use cosmos_core::{
    netty::{
        NettyChannelClient, client_preconnect_messages::ClientPreconnectMessages, connection_config, cosmos_encoder,
        sync::mapping::NetworkMapping,
    },
    state::GameState,
};
use renet::DisconnectReason;
use renet_steam::SteamClientTransport;
use serde::{Deserialize, Serialize};

use crate::{
    netty::{
        lobby::{ClientLobby, MostRecentTick},
        steam::new_steam_transport,
    },
    ui::main_menu::MainMenuSubState,
};

use super::steam::User;

#[derive(Resource, Debug, Serialize, Deserialize)]
/// Used to setup the connection with the server
///
/// This must be present before entering the `GameState::Connecting` state.
pub enum ConnectToConfig {
    /// Connect via an ip address
    Ip(SocketAddr),
    /// Connect via their steam id.
    ///
    /// This CANNOT be your own steam id - steam prevents a client + server connection if they
    /// both have the same steam id ;(
    SteamId(SteamId),
}

/// Establishes a connection with the server.
///
/// Make sure the `ConnectionConfig` resource was added first.
pub fn establish_connection(
    mut commands: Commands,
    host_config: Res<ConnectToConfig>,
    steam: Res<User>,
    mut state_changer: ResMut<NextState<GameState>>,
) {
    // match client.as_ref() {
    //     User::Steam(steam_client) => {
    //         let user = steam_client.user();
    //         let (ticket, handle) = user.authentication_session_ticket_with_steam_id(user.steam_id());
    //     }
    //     User::NoAuth(name) => {}
    // }

    let steam_transport = match new_steam_transport(steam.client().clone(), &host_config) {
        Ok(t) => t,
        Err(e) => {
            error!("{e:?}");
            state_changer.set(GameState::MainMenu);
            commands.insert_resource(MainMenuSubState::Disconnect);
            commands.insert_resource(ClientDisconnectReason(DisconnectReason::Transport));
            return;
        }
    };

    let mut client = RenetClient::new(connection_config());

    client.send_message(
        NettyChannelClient::PreConnect,
        cosmos_encoder::serialize(&ClientPreconnectMessages::Init {
            name: steam.client().friends().get_friend(steam.steam_id()).name(),
        }),
    );

    info!("Establishing connection w/ server...");
    commands.insert_resource(ClientLobby::default());
    commands.insert_resource(MostRecentTick(None));
    commands.insert_resource(client);
    commands.insert_resource(steam_transport);
    commands.init_resource::<NetworkMapping>();
    commands.remove_resource::<ClientDisconnectReason>();
}

/// Waits for a connection to be made, then changes the game state to `GameState::LoadingWorld`.
pub fn wait_for_connection(mut state_changer: ResMut<NextState<GameState>>, client: Res<RenetClient>) {
    if client.is_connected() {
        info!("Loading server data...");
        state_changer.set(GameState::LoadingData);
    }
}

fn ensure_connected(client: Res<RenetClient>, mut commands: Commands, mut state_changer: ResMut<NextState<GameState>>) {
    if client.is_disconnected() {
        commands.insert_resource(MainMenuSubState::Disconnect);
        state_changer.set(GameState::MainMenu);
    }
}

#[derive(Resource, Debug)]
/// If the renet client provides a reason for the latest disconnect, this will contain it.
pub struct ClientDisconnectReason(pub DisconnectReason);

fn remove_networking_resources(mut commands: Commands, client: Option<Res<RenetClient>>) {
    if let Some(client) = client
        && let Some(dc_reason) = client.disconnect_reason()
    {
        commands.insert_resource(ClientDisconnectReason(dc_reason));
    }
    commands.remove_resource::<NetworkMapping>();
    commands.remove_resource::<RenetClient>();
    commands.remove_resource::<SteamClientTransport>();
    commands.remove_resource::<MostRecentTick>();
    commands.remove_resource::<ClientLobby>();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, ensure_connected.run_if(in_state(GameState::LoadingData)))
        .add_systems(Update, remove_networking_resources.run_if(in_state(GameState::MainMenu)));
}
