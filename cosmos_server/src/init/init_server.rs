use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::renet::{RenetServer, ServerAuthentication, ServerConfig};
use cosmos_core::netty::{get_local_ipaddress, server_connection_config, PROTOCOL_ID};

use crate::netty::network_helpers::{ClientTicks, NetworkTick, ServerLobby};

pub fn init(app: &mut App) {
    let port: u16 = 1337;

    let local_addr = get_local_ipaddress();

    let address: SocketAddr = format!("{local_addr}:{port}").parse().unwrap();
    let socket = UdpSocket::bind(format!("0.0.0.0:{port}")).unwrap();
    socket
        .set_nonblocking(true)
        .expect("Cannot set non-blocking mode!");

    let server_config = ServerConfig::new(20, PROTOCOL_ID, address, ServerAuthentication::Unsecure);
    let connection_config = server_connection_config(); //RenetConnectionConfig::default();
    let cur_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let server = RenetServer::new(cur_time, server_config, connection_config, socket).unwrap();

    app.insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server);

    println!("Setup server on {local_addr}:{port}");
}
