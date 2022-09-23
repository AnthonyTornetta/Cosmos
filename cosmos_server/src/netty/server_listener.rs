use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::blocks::Blocks,
    entities::player::Player,
    events::block_events::BlockChangedEvent,
    netty::{
        client_reliable_messages::ClientReliableMessages,
        client_unreliable_messages::ClientUnreliableMessages, netty::NettyChannel,
        server_reliable_messages::ServerReliableMessages,
    },
    structure::{events::StructureCreated, ship::ship_builder::TShipBuilder, structure::Structure},
};

use crate::{
    events::blocks::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent},
    structure::ship::server_ship_builder::ServerShipBuilder,
};

use super::netty::ServerLobby;

fn server_listen_messages(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&mut Transform, &mut Velocity), With<Player>>,
    structure_query: Query<&Structure>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
    blocks: Res<Blocks>,
    mut block_changed_event_writer: EventWriter<BlockChangedEvent>,
    mut structure_created_event: EventWriter<StructureCreated>,
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
                    block_interact_event.send(BlockInteractEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                    });
                }
                ClientReliableMessages::CreateShip { name: _name } => {
                    let mut entity = commands.spawn();

                    let (transform, _) = players
                        .get(lobby.players.get(&client_id).unwrap().clone())
                        .unwrap();

                    let mut structure = Structure::new(10, 10, 10, entity.id());

                    let builder = ServerShipBuilder::default();

                    let mut ship_transform = transform.clone();
                    ship_transform.translation.z += 4.0;

                    builder.insert_ship(
                        &mut entity,
                        ship_transform,
                        Velocity::zero(),
                        &mut structure,
                    );

                    let block = blocks.block_from_id("cosmos:ship_core");

                    structure.set_block_at(
                        structure.blocks_width() / 2,
                        structure.blocks_height() / 2,
                        structure.blocks_length() / 2,
                        block,
                        &blocks,
                        Some(&mut block_changed_event_writer),
                    );

                    entity.insert(structure);

                    structure_created_event.send(StructureCreated {
                        entity: entity.id(),
                    });
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_listen_messages);
}
