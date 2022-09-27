use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{
        client_reliable_messages::ClientReliableMessages,
        client_unreliable_messages::ClientUnreliableMessages, netty::NettyChannel,
        server_reliable_messages::ServerReliableMessages,
    },
    structure::structure::{Structure, StructureBlock},
};

use crate::events::{
    blocks::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent},
    create_ship_event::CreateShipEvent,
};

use super::netty::ServerLobby;

fn server_listen_messages(
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&mut Transform, &mut Velocity), With<Player>>,
    structure_query: Query<&Structure>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
    mut create_ship_event_writer: EventWriter<CreateShipEvent>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannel::Unreliable.id()) {
            let command: ClientUnreliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientUnreliableMessages::PlayerBody { body } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok((mut transform, mut velocity)) = players.get_mut(*player_entity) {
                            transform.translation = body.translation.clone().into();
                            transform.rotation = body.rotation.clone().into();

                            velocity.linvel = body.body_vel.linvel.into();
                            velocity.angvel = body.body_vel.angvel.into();
                        }
                    }
                }
            }
        }

        while let Some(message) = server.receive_message(client_id, NettyChannel::Reliable.id()) {
            let command: ClientReliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientReliableMessages::PlayerDisconnect => {}
                ClientReliableMessages::SendChunk { server_entity } => {
                    let entity = structure_query.get(server_entity.clone());

                    if entity.is_ok() {
                        let structure = entity.unwrap();

                        for chunk in structure.chunks() {
                            server.send_message(
                                client_id,
                                NettyChannel::Reliable.id(),
                                bincode::serialize(&ServerReliableMessages::ChunkData {
                                    structure_entity: server_entity.clone(),
                                    serialized_chunk: bincode::serialize(chunk).unwrap(),
                                })
                                .unwrap(),
                            );
                        }
                    } else {
                        println!(
                            "!!! Server received invalid entity from client {}",
                            client_id
                        );
                    }
                }
                ClientReliableMessages::BreakBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                } => {
                    break_block_event.send(BlockBreakEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                    });
                }
                ClientReliableMessages::PlaceBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                    block_id,
                } => {
                    place_block_event.send(BlockPlaceEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                        block_id,
                    });
                }
                ClientReliableMessages::InteractWithBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                } => {
                    println!("SENT EVENT!");
                    block_interact_event.send(BlockInteractEvent {
                        structure_entity,
                        structure_block: StructureBlock::new(x, y, z),
                        interactor: lobby.players.get(&client_id).unwrap().clone(),
                    });
                }
                ClientReliableMessages::CreateShip { name: _name } => {
                    let (transform, _) = players
                        .get(lobby.players.get(&client_id).unwrap().clone())
                        .unwrap();

                    let mut ship_transform = transform.clone();
                    ship_transform.translation.z += 4.0;

                    create_ship_event_writer.send(CreateShipEvent { ship_transform });
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_listen_messages);
}
