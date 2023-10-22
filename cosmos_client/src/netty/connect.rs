//! Contains all the connection functionality from client -> server
//!
//! This does not add them to the bevy systems by default, and they must be manually added when needed.

use std::{
    net::UdpSocket,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;
use bevy_renet::renet::{
    transport::{ClientAuthentication, NetcodeClientTransport},
    RenetClient,
};
use cosmos_core::{
    entities::player::Player,
    netty::{connection_config, PROTOCOL_ID},
};

use crate::{
    netty::{
        lobby::{ClientLobby, MostRecentTick},
        mapping::NetworkMapping,
    },
    state::game_state::GameState,
};

use super::flags::LocalPlayer;

fn new_netcode_transport(host: &str) -> NetcodeClientTransport {
    let port: u16 = 1337;

    let server_addr = format!("{host}:{port}").parse().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();

    socket.set_nonblocking(true).expect("Unable to make UDP non-blocking!");

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;

    let name = "CoolPlayer";

    let mut token = [0; 256];

    // Bincode because this is stored un a u8, with a fixed length of 256
    let serialized_name = bincode::serialize(&name).expect("Unable to serialize name");
    for (i, byte) in serialized_name.iter().enumerate() {
        token[i] = *byte;
    }

    let auth = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: PROTOCOL_ID,
        server_addr,
        user_data: Some(token),
    };

    info!("Connecting to {server_addr}");

    NetcodeClientTransport::new(current_time, auth, socket).unwrap()
}

#[derive(Resource)]
/// Used to setup the connection with the server
pub struct HostConfig {
    /// The server's host
    pub host_name: String,
}

/// Establishes a connection with the server.
///
/// Make sure the `ConnectionConfig` resource was added first.
pub fn establish_connection(mut commands: Commands, host_config: Res<HostConfig>) {
    info!("Establishing connection w/ server...");
    commands.insert_resource(ClientLobby::default());
    commands.insert_resource(MostRecentTick(None));
    commands.insert_resource(RenetClient::new(connection_config()));
    commands.insert_resource(new_netcode_transport(host_config.host_name.as_str()));
    commands.insert_resource(NetworkMapping::default());
}

/// Waits for a connection to be made, then changes the game state to `GameState::LoadingWorld`.
pub fn wait_for_connection(mut state_changer: ResMut<NextState<GameState>>, transport: Res<NetcodeClientTransport>) {
    if transport.is_connected() {
        info!("Loading server data...");
        state_changer.set(GameState::LoadingWorld);
    }
}

/// Waits for the `LoadingWorld` state to be done loading, then transitions to the `GameState::Playing`
pub fn wait_for_done_loading(mut state_changer: ResMut<NextState<GameState>>, query: Query<&Player, With<LocalPlayer>>) {
    if query.get_single().is_ok() {
        info!("Got local player, starting game!");
        state_changer.set(GameState::Playing);
    }
}
