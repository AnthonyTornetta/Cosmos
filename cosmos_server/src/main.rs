mod server;

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;
use bevy::prelude::*;
use bevy::winit::WinitPlugin;
use bevy_rapier3d::prelude::Velocity;
use bevy_rapier3d::rapier::prelude::RigidBodyVelocity;
use bevy_renet::renet::{RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};
use bevy_renet::RenetServerPlugin;
use cosmos_core::entities::player::Player;
use cosmos_core::netty::netty::{ClientUnreliableMessages, NettyChannel, server_connection_config, ServerUnreliableMessages};
use cosmos_core::netty::netty::ServerUnreliableMessages::{BulkBodies};
use cosmos_core::netty::netty_rigidbody::NettyRigidBody;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

#[derive(Debug, Default)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

#[derive(Debug, Default)]
pub struct NetworkTick (u32);

fn handle_messages(mut server: ResMut<RenetServer>)
{
    let channel_id = 0;

    for client_id in server.clients_id().into_iter()
    {
        while let Some(message) = server.receive_message(client_id, channel_id)
        {

        }
    }
}

fn handle_events_system(mut server: ResMut<RenetServer>, mut server_events: EventReader<ServerEvent>) {
    while let Some(event) = server.get_event() {
        for event in server_events.iter() {
            match event {
                ServerEvent::ClientConnected(id, user_data) => {
                    println!("Client {} connected", id);
                }
                ServerEvent::ClientDisconnected(id) => {
                    println!("Client {} disconnected", id);
                }
            }
        }
    }
}

fn send_message_system(mut server: ResMut<RenetServer>) {
    let channel_id = 0;
    // Send a text message for all clients
    server.broadcast_message(channel_id, "server message".as_bytes().to_vec());
}

const PROTOCOL_ID: u64 = 7;

fn server_sync_bodies(
    mut server: ResMut<RenetServer>,
    mut tick: ResMut<NetworkTick>,
    players: Query<(Entity, &Transform, &Velocity)>) {

    let mut bodies = Vec::new();

    for (entity, transform, velocity) in players.iter() {
        bodies.push((entity.clone(), NettyRigidBody::new(&velocity, &transform)));
    }


    tick.0 += 1;

    let mut sync_message = BulkBodies {
        time_stamp: tick.0,
        bodies
    };
    let message = bincode::serialize(&sync_message).unwrap();

    server.broadcast_message(NettyChannel::Unreliable.id(), message);
}

fn server_listen_messages(
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<ServerLobby>,
    mut players: Query<(&mut Transform, &mut Velocity), With<Player>>) {

    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannel::Unreliable.id()) {
            let command: ClientUnreliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientUnreliableMessages::PlayerBody { body } => {
                    println!("Player transform received from {}!", client_id);

                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok((mut transform, mut velocity)) = players.get_mut(*player_entity) {
                            transform.translation = body.translation.into();
                            transform.rotation = body.rotation.into();
                            velocity.linvel = body.body_vel.linvel.into();
                            velocity.angvel = body.body_vel.angvel.into();
                        }
                   }
                }
            }
        }
    }
}

fn main() {

    let port: u16 = 1337;

    let address: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let socket = UdpSocket::bind(address).unwrap();

    let server_config = ServerConfig::new(20, PROTOCOL_ID, address, ServerAuthentication::Unsecure);
    let connection_config = server_connection_config(); //RenetConnectionConfig::default();
    let cur_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let server = RenetServer::new(cur_time, server_config, connection_config, socket).unwrap();

    App::new()
        .add_plugins(CosmosCorePluginGroup::default())
        .add_plugin(RenetServerPlugin)
        .add_plugin(WinitPlugin::default())

        .insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(server)

        .add_system(handle_messages)
        .add_system(handle_events_system)
        .add_system(send_message_system)
        .run();
}
