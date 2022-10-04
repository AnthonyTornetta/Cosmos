use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{
        client_reliable_messages::ClientReliableMessages,
        client_unreliable_messages::ClientUnreliableMessages,
        server_reliable_messages::ServerReliableMessages, NettyChannel,
    },
    structure::{
        ship::pilot::Pilot,
        structure::{Structure, StructureBlock},
    },
};

use crate::events::{
    blocks::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent},
    create_ship_event::CreateShipEvent,
    structure::ship::ShipSetMovementEvent,
};

use super::network_helpers::ServerLobby;

fn server_listen_messages(
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&mut Transform, &mut Velocity), With<Player>>,
    structure_query: Query<&Structure>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
    mut create_ship_event_writer: EventWriter<CreateShipEvent>,

    mut ship_movement_event_writer: EventWriter<ShipSetMovementEvent>,
    pilot_query: Query<&Pilot>,
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
                ClientUnreliableMessages::SetMovement { movement } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok(pilot) = pilot_query.get(*player_entity) {
                            let ship = pilot.entity;

                            ship_movement_event_writer
                                .send(ShipSetMovementEvent { movement, ship });
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
                    if let Ok(structure) = structure_query.get(server_entity) {
                        for chunk in structure.chunks() {
                            server.send_message(
                                client_id,
                                NettyChannel::Reliable.id(),
                                bincode::serialize(&ServerReliableMessages::ChunkData {
                                    structure_entity: server_entity,
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
                        structure_block: StructureBlock::new(x, y, z),
                        interactor: *lobby.players.get(&client_id).unwrap(),
                    });
                }
                ClientReliableMessages::CreateShip { name: _name } => {
                    let (transform, _) = players
                        .get(*lobby.players.get(&client_id).unwrap())
                        .unwrap();

                    let mut ship_transform = *transform;
                    ship_transform.translation.z += 4.0;

                    create_ship_event_writer.send(CreateShipEvent { ship_transform });
                }
                ClientReliableMessages::PilotQuery { ship_entity } => {
                    let pilot = match pilot_query.get(ship_entity) {
                        Ok(pilot) => Some(pilot.entity),
                        _ => None,
                    };

                    server.send_message(
                        client_id,
                        NettyChannel::Reliable.id(),
                        bincode::serialize(&ServerReliableMessages::PilotChange {
                            structure_entity: ship_entity,
                            pilot_entity: pilot,
                        })
                        .unwrap(),
                    );
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_listen_messages);
}
