use std::time::SystemTime;
use bevy::prelude::*;
use bevy_renet::renet::{RenetServer, ServerConfig, ServerEvent};
use bevy_renet::RenetServerPlugin;
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;

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

fn handle_events_system(mut server_events: EventReader<ServerEvent>) {
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

fn main() {
    let server = RenetServer::new();

    App::new()
        .add_plugins(CosmosCorePluginGroup::default())
        .add_plugin(RenetServerPlugin)
        .insert_resource(server)
        .add_system(handle_messages)
        .add_system(handle_events_system)
        .add_system(send_message_system)
    ;
}
