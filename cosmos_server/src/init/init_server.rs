//! Sets up the server & makes it ready to be connected to.
//!
//! Use `init` to do this.

use bevy::prelude::*;

use bevy_renet::{
    renet::RenetServer,
    steam::steamworks::{
        Client,
        networking_types::{NetworkingConfigEntry, NetworkingConfigValue},
    },
};
use cosmos_core::netty::{connection_config, server::ServerLobby};
use renet_steam::{SteamServerConfig, SteamServerSocketOptions, SteamServerTransport};

use crate::netty::network_helpers::{ClientTicks, NetworkTick};

#[derive(Resource)]
/// Stores the steam [`Client`] used by the server
pub struct ServerSteamClient {
    client: Client,
}

impl ServerSteamClient {
    /// Returns the steam [`Client`] used by the server
    pub fn client(&self) -> &Client {
        &self.client
    }
}

/// Sets up the server & makes it ready to be connected to
pub fn init(app: &mut App, port: u16) {
    // let public_addr = format!("0.0.0.0:{port}").parse().unwrap();
    // let socket = UdpSocket::bind(public_addr).unwrap();

    // let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    // let config = ServerSocketConfig {
    //     needs_encryption: false,
    //     public_addresses: vec![public_addr],
    // };

    let setup_config = SteamServerConfig {
        access_permission: renet_steam::AccessPermission::Public,
        // current_time,
        max_clients: 64,
        // protocol_id: PROTOCOL_ID,
        // public_addresses: vec![public_addr],
        // authentication: ServerAuthentication::Unsecure,
    };

    // let server_config = ServerConfig {
    //     max_clients: 20,
    //     protocol_id: PROTOCOL_ID,
    //     sockets: vec![config],
    //     current_time,
    //     authentication: ServerAuthentication::Unsecure,
    // };

    info!("Creating steam server...");

    // let (steam_server, c) =
    //     renet_steam::steamworks::Server::init("0.0.0.0".parse().unwrap(), port, port + 1, ServerMode::Authentication, "0.0.9a").unwrap();
    //
    //     info!("Created steam server!");
    //
    //     commands.insert_resource(AuthenticationServer::Steam(steam_server));

    let steam_client = Client::init().unwrap();
    info!("Server steam id: {:?}", steam_client.user().steam_id());
    let netty = steam_client.networking_utils();
    netty.init_relay_network_access();

    const MEGABYTE: i32 = 1024 * 1024;
    let socket_options = SteamServerSocketOptions::default()
        .with_address(format!("0.0.0.0:{port}").parse().unwrap())
        .with_config(NetworkingConfigEntry::new_int32(
            NetworkingConfigValue::SendBufferSize,
            10 * MEGABYTE,
        ));

    /*
        *
        let socket_options = SteamServerSocketOptions::default()
            .with_address(format!("0.0.0.0:{port}").parse().unwrap())
            .with_config(NetworkingConfigEntry::new_int32(
                NetworkingConfigValue::SendBufferSize,
                10 * MEGABYTE,
            ));
    */

    let transport = SteamServerTransport::new(&steam_client, setup_config, socket_options).unwrap();
    let server = RenetServer::new(connection_config());

    app.insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server)
        .insert_non_send_resource(transport)
        .insert_resource(ServerSteamClient { client: steam_client });

    info!("Steam server created!");

    // info!("Public address: {public_addr}");
}

fn steam_callbacks(client: Option<Res<ServerSteamClient>>) {
    if let Some(client) = client {
        client.client.run_callbacks();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreUpdate, steam_callbacks);
}
