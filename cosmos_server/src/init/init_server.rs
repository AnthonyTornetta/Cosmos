//! Sets up the server & makes it ready to be connected to.
//!
//! Use `init` to do this.

use std::{net::UdpSocket, time::SystemTime};

use bevy::prelude::*;
use bevy_renet::renet::{
    transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    RenetServer,
};
use cosmos_core::netty::{connection_config, server::ServerLobby, PROTOCOL_ID};

use crate::netty::network_helpers::{ClientTicks, NetworkTick};

/// Sets up the server & makes it ready to be connected to
pub fn init(app: &mut App, port: u16) {
    let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).unwrap();

    socket.set_nonblocking(true).expect("Cannot set non-blocking mode!");

    let public_addr = socket.local_addr().unwrap();

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let server_config = ServerConfig {
        max_clients: 20,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        current_time,
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
    let server = RenetServer::new(connection_config());

    app.insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server)
        .insert_resource(transport);

    info!("Public address: {public_addr}");
}
