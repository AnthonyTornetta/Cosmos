//! Sets up the server & makes it ready to be connected to.
//!
//! Use `init` to do this.

use std::{net::UdpSocket, time::SystemTime};

use bevy::prelude::*;
use bevy_renet::{
    netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::RenetServer,
};
use cosmos_core::netty::{PROTOCOL_ID, connection_config, server::ServerLobby};

use crate::netty::network_helpers::{ClientTicks, NetworkTick};

/// Sets up the server & makes it ready to be connected to
pub fn init(app: &mut App, port: u16) {
    let public_addr = format!("0.0.0.0:{port}").parse().unwrap();
    let socket = UdpSocket::bind(public_addr).unwrap();

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    // let config = ServerSocketConfig {
    //     needs_encryption: false,
    //     public_addresses: vec![public_addr],
    // };

    let setup_config = ServerConfig {
        current_time,
        max_clients: 64,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    // let server_config = ServerConfig {
    //     max_clients: 20,
    //     protocol_id: PROTOCOL_ID,
    //     sockets: vec![config],
    //     current_time,
    //     authentication: ServerAuthentication::Unsecure,
    // };

    let transport = NetcodeServerTransport::new(setup_config, socket).unwrap();
    let server = RenetServer::new(connection_config());

    app.insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server)
        .insert_resource(transport);

    info!("Public address: {public_addr}");
}
