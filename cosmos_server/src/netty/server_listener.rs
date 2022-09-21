use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::netty::{
        ClientReliableMessages, ClientUnreliableMessages, NettyChannel, ServerReliableMessages,
    },
    structure::structure::Structure,
};

use crate::events::blocks::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent};

use super::netty::ServerLobby;

fn server_listen_messages(
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&mut Transform, &mut Velocity), With<Player>>,
    structure_query: Query<&Structure>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannel::Unreliable.id()) {
            let command: ClientUnreliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientUnreliableMessages::PlayerBody { body } => {
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

        while let Some(message) = server.receive_message(client_id, NettyChannel::Reliable.id()) {
            let command: ClientReliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientReliableMessages::PlayerDisconnect => {}
                ClientReliableMessages::SendChunk { server_entity } => {
                    let structure = structure_query.get(server_entity.clone()).unwrap();

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
                    block_interact_event.send(BlockInteractEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                    });
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_listen_messages);
}
