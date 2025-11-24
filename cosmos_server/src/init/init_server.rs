//! Sets up the server & makes it ready to be connected to.
//!
//! Use `init` to do this.

use std::net::UdpSocket;

use bevy::prelude::*;

use bevy_renet::{
    renet::RenetServer,
    steam::steamworks::{
        Client, Server,
        networking_types::{NetworkingConfigEntry, NetworkingConfigValue},
    },
};
use cosmos_core::netty::{connection_config, server::ServerLobby};
use renet_steam::{SteamServerConfig, SteamServerSocketOptions, SteamServerTransport};

use crate::{
    local::LocalServer,
    netty::network_helpers::{ClientTicks, NetworkTick},
    plugin::server_plugin::ServerType,
};

#[derive(Resource)]
/// Stores the steam [`Client`] used by the server
///
/// If the server is dedicated, then `Server` will also be set.
pub struct ServerSteamClient {
    client: Client,
    server: Option<Server>,
}

impl ServerSteamClient {
    /// Returns the steam [`Client`] used by the server
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the steam [`Server`] used by the server
    pub fn server(&self) -> Option<&Server> {
        self.server.as_ref()
    }
}

/// Sets up the server & makes it ready to be connected to
pub fn init(app: &mut App, server_type: &ServerType) {
    // let public_addr = format!("0.0.0.0:{port}").parse().unwrap();
    // let socket = UdpSocket::bind(public_addr).unwrap();

    // let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    // let config = ServerSocketConfig {
    //     needs_encryption: false,
    //     public_addresses: vec![public_addr],
    // };

    info!("Creating steam server ({server_type:?})...");

    match server_type {
        ServerType::Dedicated { port } => {
            create_dedicated_server(app, *port);
        }
        ServerType::Local => {
            create_local_server(app);
        }
    }

    info!("Steam server created!");
}

const MEGABYTE: i32 = 1024 * 1024;

fn create_local_server(app: &mut App) {
    let setup_config = SteamServerConfig {
        access_permission: renet_steam::AccessPermission::Public,
        max_clients: 64,
    };

    let steam_client = Client::init().unwrap();

    info!("Server steam id: {:?}", steam_client.user().steam_id());

    let netty = steam_client.networking_utils();
    netty.init_relay_network_access();

    // Steam requires us to pass a valid port instead of letting the os choose one. so I let the OS
    // choose one here, then pass that port to steam. This is mega jank, so hopefully I find a
    // better way
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);

    let socket_options = SteamServerSocketOptions::default()
        .with_address(format!("0.0.0.0:{port}").parse().unwrap())
        .with_config(NetworkingConfigEntry::new_int32(
            NetworkingConfigValue::SendBufferSize,
            10 * MEGABYTE,
        ))
        // Just a big number, we should find a value using science later. If this is too small,
        // the client can't process the server's messages fast enough and it stalls out
        //
        // SERVER NOTE: idk if this is even needed for the server.
        .with_max_batch_size(100000);

    // THIS NEEDS TO STAY:
    // The client reads this value and uses that to get the correct port!
    println!("Port: {port}");

    let transport = SteamServerTransport::new(steam_client.clone(), setup_config, socket_options).unwrap();
    let server = RenetServer::new(connection_config());

    app.insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server)
        .insert_resource(LocalServer)
        .insert_non_send_resource(transport)
        .insert_resource(ServerSteamClient {
            client: steam_client,
            server: None,
        });
}

fn create_dedicated_server(app: &mut App, port: u16) {
    let setup_config = SteamServerConfig {
        access_permission: renet_steam::AccessPermission::Public,
        max_clients: 64,
    };

    let (steam_server, steam_client) = match Server::init(
        "0.0.0.0".parse().unwrap(),
        port,
        port + 1,
        bevy_renet::steam::steamworks::ServerMode::Authentication,
        "0.0.9",
    ) {
        Ok((server, client)) => (server, client),
        Err(e) => {
            panic!("Couldn't start server! {e:?}");
        }
    };

    info!("Created server!");

    steam_server.set_product(steam_server.utils().app_id().0.to_string().as_str());
    steam_server.set_game_description("Vanilla Cosmos Server");
    steam_server.set_max_players(32);
    steam_server.set_server_name("My Cool Cosmos Server");
    steam_server.set_dedicated_server(true);

    steam_server.set_advertise_server_active(true);
    steam_server.log_on_anonymous();

    // steam_server.networking_messages().session_request_callback(|req| {
    //     info!("REQ");
    //     req.accept();
    // });
    //
    // let _cb1 = steam_server.register_callback(|_: SteamServersConnected| {
    //     info!("Steam servers connected");
    // });
    //
    // let _cb2 = steam_server.register_callback(|_: SteamServerConnectFailure| {
    //     error!("Steam server connect failure");
    // });
    //
    // let _cb3 = steam_server.register_callback(|_: SteamServersDisconnected| {
    //     error!("Steam servers disconnected");
    // });

    info!("Server steam id: {:?}", steam_server.steam_id());

    let netty = steam_client.networking_utils();
    netty.init_relay_network_access();

    let socket_options = SteamServerSocketOptions::default()
        .with_address(format!("0.0.0.0:{port}").parse().unwrap())
        .with_config(NetworkingConfigEntry::new_int32(
            NetworkingConfigValue::SendBufferSize,
            10 * MEGABYTE,
        ))
        // Just a big number, we should find a value using science later. If this is too small,
        // the client can't process the server's messages fast enough and it stalls out
        //
        // SERVER NOTE: idk if this is even needed for the server.
        .with_max_batch_size(100000);

    let transport = SteamServerTransport::new_server(steam_server.clone(), steam_client.clone(), setup_config, socket_options).unwrap();
    let server = RenetServer::new(connection_config());

    app.insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server)
        .insert_non_send_resource(transport)
        .insert_resource(ServerSteamClient {
            client: steam_client,
            server: Some(steam_server),
        });

    // app.insert_non_send_resource(_cb1);
    // app.insert_non_send_resource(_cb2);
    // app.insert_non_send_resource(_cb3);
}

fn steam_callbacks(steam: Option<Res<ServerSteamClient>>) {
    if let Some(steam) = steam {
        steam.client.run_callbacks();
        if let Some(server) = steam.server.as_ref() {
            server.run_callbacks();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreUpdate, steam_callbacks);
}
