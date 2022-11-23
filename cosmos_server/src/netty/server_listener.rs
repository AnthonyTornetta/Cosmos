use bevy::prelude::*;
use bevy_rapier3d::prelude::ReadMassProperties;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    entities::player::Player,
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{
        client_reliable_messages::ClientReliableMessages,
        client_unreliable_messages::ClientUnreliableMessages,
        server_reliable_messages::ServerReliableMessages, NettyChannel,
    },
    structure::{
        ship::pilot::Pilot,
        structure::{Structure, StructureBlock, StructureShape},
    },
};

use crate::events::{
    blocks::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent},
    create_ship_event::CreateShipEvent,
    structure::ship::ShipSetMovementEvent,
};

use super::network_helpers::ServerLobby;

fn server_listen_messages(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    players: Query<Entity, With<Player>>,
    transform_query: Query<&Transform>,
    structure_query: Query<&Structure>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
    mut create_ship_event_writer: EventWriter<CreateShipEvent>,

    mut ship_movement_event_writer: EventWriter<ShipSetMovementEvent>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
    pilot_query: Query<&Pilot>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannel::Unreliable.id()) {
            let command: ClientUnreliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientUnreliableMessages::PlayerBody { body } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok(entity) = players.get(*player_entity) {
                            commands
                                .entity(entity)
                                .insert(TransformBundle::from_transform(body.create_transform()))
                                .insert(ReadMassProperties::default());
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
                        match structure.shape() {
                            StructureShape::Flat => {
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
                            }
                            StructureShape::Sphere { radius } => {
                                let client_ent = lobby.players.get(&client_id).unwrap();
                                let transform = transform_query.get(*client_ent).unwrap();
                                let structure_transform =
                                    transform_query.get(server_entity).unwrap();

                                let delta_location =
                                    transform.translation - structure_transform.translation;

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
                            }
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
                    let transform = transform_query
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
                ClientReliableMessages::StopPiloting => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok(piloting) = pilot_query.get(*player_entity) {
                            pilot_change_event_writer.send(ChangePilotEvent {
                                structure_entity: piloting.entity,
                                pilot_entity: None,
                            });
                        }
                    }
                }
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_listen_messages);
}
