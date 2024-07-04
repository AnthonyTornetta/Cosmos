//! Sets up the server & makes it ready to be connected to.
//!
//! Use `init` to do this.

use std::{
    net::{ToSocketAddrs, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::renet::{
    transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    RenetServer,
};
use cosmos_core::netty::{connection_config, get_local_ipaddress, server::ServerLobby, PROTOCOL_ID};

use crate::netty::network_helpers::{ClientTicks, NetworkTick};

/// Sets up the server & makes it ready to be connected to
pub fn init(app: &mut App, address: Option<String>, port: u16) {
    // let addr = address.unwrap_or(get_local_ipaddress());

    // let public_addr = format!("{addr}:{port}")
    //     .to_socket_addrs()
    //     .unwrap_or_else(|e| panic!("Error creating IP address for \"{addr}\". Error: {e:?}"))
    //     .next()
    //     .unwrap();

    let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).unwrap();
    info!("Server Local Addr: {:?}", socket.local_addr());

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
