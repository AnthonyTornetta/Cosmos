//! Contains all the connection functionality from client -> server
//!
//! This does not add them to the bevy systems by default, and they must be manually added when needed.

use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;
use bevy_renet2::renet2::{
    transport::{ClientAuthentication, NetcodeClientTransport},
    RenetClient,
};
use cosmos_core::{
    entities::player::Player,
    netty::{client::LocalPlayer, connection_config, sync::mapping::NetworkMapping, PROTOCOL_ID},
};
use renet2::transport::NativeSocket;

use crate::{
    netty::lobby::{ClientLobby, MostRecentTick},
    state::game_state::GameState,
};

fn new_netcode_transport(mut host: &str, port: u16) -> NetcodeClientTransport {
    if host == "localhost" {
        host = "127.0.0.1"; // to_socket_addrs turns localhost into an ipv6 IP, which fails to connect to the server listening on an ipv4 address.
    }

    let addr = format!("{host}:{port}");

    let server_addr = addr
        .to_socket_addrs()
        .unwrap_or_else(|e| panic!("Error creating IP address for \"{addr}\". Error: {e:?}"))
        .next()
        .unwrap();

    let socket = NativeSocket::new(UdpSocket::bind("0.0.0.0:0").unwrap()).unwrap();

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
        socket_id: 0, // for native sockets, use 0
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
///
/// This must be present before entering the `GameState::Connecting` state.
pub struct HostConfig {
    /// The server's host (excluding port)
    pub host_name: String,
    /// The server's port
    pub port: u16,
}

/// Establishes a connection with the server.
///
/// Make sure the `ConnectionConfig` resource was added first.
pub fn establish_connection(mut commands: Commands, host_config: Res<HostConfig>) {
    info!("Establishing connection w/ server...");
    commands.insert_resource(ClientLobby::default());
    commands.insert_resource(MostRecentTick(None));
    commands.insert_resource(RenetClient::new(connection_config()));
    commands.insert_resource(new_netcode_transport(host_config.host_name.as_str(), host_config.port));
    commands.init_resource::<NetworkMapping>();
}

/// Waits for a connection to be made, then changes the game state to `GameState::LoadingWorld`.
pub fn wait_for_connection(mut state_changer: ResMut<NextState<GameState>>, client: Res<RenetClient>) {
    if client.is_connected() {
        info!("Loading server data...");
        state_changer.set(GameState::LoadingData);
    }
}

#[derive(Component)]
/// Add this component to an entity to ensure the state isn't advanced to playing. Remove this when you're ready to start playing.
pub struct WaitingOnServer;

// GameState::LoadingData -> GameState::LoadingWorld in registry/mod.rs

/// Waits for the `LoadingWorld` state to be done loading, then transitions to the `GameState::Playing`
pub fn wait_for_done_loading(
    mut state_changer: ResMut<NextState<GameState>>,
    q_waiting: Query<(), With<WaitingOnServer>>,
    query: Query<&Player, With<LocalPlayer>>,
) {
    if !q_waiting.is_empty() {
        return;
    }

    if query.get_single().is_ok() {
        info!("Got local player, starting game!");
        state_changer.set(GameState::Playing);
    }
}
