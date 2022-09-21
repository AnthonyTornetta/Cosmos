use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};

use bevy::prelude::*;
use bevy_renet::renet::{RenetServer, ServerAuthentication, ServerConfig};
use cosmos_core::netty::netty::{server_connection_config, PROTOCOL_ID};

use crate::netty::netty::{ClientTicks, NetworkTick, ServerLobby};

pub fn init(app: &mut App) {
    let port: u16 = 1337;

    let address: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let socket = UdpSocket::bind(address).unwrap();

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
}
