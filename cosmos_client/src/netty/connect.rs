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
    netty::{connection_config, sync::mapping::NetworkMapping, PROTOCOL_ID},
    state::GameState,
};
use renet2::transport::NativeSocket;

use crate::{
    netty::lobby::{ClientLobby, MostRecentTick},
    ui::main_menu::MainMenuSubState,
};

fn new_netcode_transport(player_name: &str, mut host: &str, port: u16) -> NetcodeClientTransport {
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

    let mut token = [0; 256];

    // Bincode because this is stored un a u8, with a fixed length of 256
    let serialized_name = bincode::serialize(&player_name).expect("Unable to serialize name");
    if serialized_name.len() > 256 {
        panic!("name too long. TODO: Handle this gracefully");
    }

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
    /// The player's name
    pub name: String,
}

/// Establishes a connection with the server.
///
/// Make sure the `ConnectionConfig` resource was added first.
pub fn establish_connection(mut commands: Commands, host_config: Res<HostConfig>) {
    info!("Establishing connection w/ server...");
    commands.insert_resource(ClientLobby::default());
    commands.insert_resource(MostRecentTick(None));
    commands.insert_resource(RenetClient::new(connection_config()));
    commands.insert_resource(new_netcode_transport(
        &host_config.name,
        host_config.host_name.as_str(),
        host_config.port,
    ));
    commands.init_resource::<NetworkMapping>();
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

fn remove_network_mapping(mut commands: Commands) {
    commands.remove_resource::<NetworkMapping>();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, ensure_connected.run_if(in_state(GameState::LoadingData)))
        .add_systems(Update, remove_network_mapping.run_if(in_state(GameState::MainMenu)));
}
