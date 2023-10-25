//! Listens to almost all the messages received from the client
//!
//! Eventually this should be broken down into more specific functions

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::netty::netty_rigidbody::NettyRigidBodyLocation;
use cosmos_core::netty::{cosmos_encoder, NettyChannelClient, NettyChannelServer};
use cosmos_core::physics::location::Location;
use cosmos_core::structure::ship::build_mode::ExitBuildModeEvent;
use cosmos_core::structure::systems::{SystemActive, Systems};
use cosmos_core::{
    entities::player::Player,
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{
        client_reliable_messages::ClientReliableMessages, client_unreliable_messages::ClientUnreliableMessages,
        server_reliable_messages::ServerReliableMessages,
    },
    structure::{ship::pilot::Pilot, Structure},
};

use crate::entities::player::PlayerLooking;
use crate::events::{
    blocks::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent},
    create_ship_event::CreateShipEvent,
    structure::ship::ShipSetMovementEvent,
};
use crate::structure::planet::generation::planet_generator::RequestChunkEvent;

use super::network_helpers::ServerLobby;
use super::sync::entities::RequestedEntityEvent;

/// Bevy system that listens to almost all the messages received from the client
///
/// Eventually this should be broken down into more specific functions
pub fn server_listen_messages(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    structure_query: Query<&Structure>,
    (
        mut systems_query,
        mut break_block_event,
        mut place_block_event,
        mut block_interact_event,
        mut exit_build_mode_writer,
        mut create_ship_event_writer,
        mut requested_entities_writer,
        mut request_chunk_event_writer,
    ): (
        Query<&mut Systems>,
        EventWriter<BlockBreakEvent>,
        EventWriter<BlockPlaceEvent>,
        EventWriter<BlockInteractEvent>,
        EventWriter<ExitBuildModeEvent>,
        EventWriter<CreateShipEvent>,
        EventWriter<RequestedEntityEvent>,
        EventWriter<RequestChunkEvent>,
    ),
    (mut ship_movement_event_writer, mut pilot_change_event_writer): (EventWriter<ShipSetMovementEvent>, EventWriter<ChangePilotEvent>),
    pilot_query: Query<&Pilot>,
    player_parent_location: Query<&Location, Without<Player>>,
    mut change_player_query: Query<(&mut Transform, &mut Location, &mut PlayerLooking, &mut Velocity), With<Player>>,
    non_player_transform_query: Query<&Transform, Without<Player>>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Unreliable) {
            if let Some(player_entity) = lobby.player_from_id(client_id) {
                let Ok(command) = cosmos_encoder::deserialize::<ClientUnreliableMessages>(&message) else {
                    warn!("UNABLE TO DESERIALIZE CLIENT MESSAGE!");
                    break;
                };

                match command {
                    ClientUnreliableMessages::PlayerBody { body, looking } => {
                        if let Ok((mut transform, mut location, mut currently_looking, mut velocity)) =
                            change_player_query.get_mut(player_entity)
                        {
                            let new_loc = match body.location {
                                NettyRigidBodyLocation::Absolute(location) => location,
                                NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                                    let parent_loc = player_parent_location.get(entity).copied().unwrap_or(Location::default());

                                    parent_loc + rel_trans
                                }
                            };

                            location.set_from(&new_loc);
                            location.last_transform_loc = Some(transform.translation);
                            currently_looking.rotation = looking;
                            velocity.linvel = body.body_vel.linvel.into();
                            transform.rotation = body.rotation;
                        }
                    }
                    ClientUnreliableMessages::SetMovement { movement } => {
                        if let Ok(pilot) = pilot_query.get(player_entity) {
                            let ship = pilot.entity;

                            ship_movement_event_writer.send(ShipSetMovementEvent { movement, ship });
                        }
                    }
                    ClientUnreliableMessages::ShipStatus { use_system } => {
                        if let Ok(pilot) = pilot_query.get(player_entity) {
                            if use_system {
                                commands.entity(pilot.entity).insert(SystemActive);
                            } else {
                                commands.entity(pilot.entity).remove::<SystemActive>();
                            }
                        }
                    }
                    ClientUnreliableMessages::ShipActiveSystem { active_system } => {
                        if let Ok(pilot) = pilot_query.get(player_entity) {
                            if let Ok(mut systems) = systems_query.get_mut(pilot.entity) {
                                systems.set_active_system(active_system, &mut commands);
                            }
                        }
                    }
                }
            }
        }

        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Reliable) {
            let Ok(command) = cosmos_encoder::deserialize::<ClientReliableMessages>(&message) else {
                warn!("UNABLE TO DESERIALIZE CLIENT MESSAGE!");
                break;
            };

            match command {
                ClientReliableMessages::SendAllChunks { server_entity } => {
                    if let Ok(structure) = structure_query.get(server_entity) {
                        for (_, chunk) in structure.chunks() {
                            server.send_message(
                                client_id,
                                NettyChannelServer::Reliable,
                                cosmos_encoder::serialize(&ServerReliableMessages::ChunkData {
                                    structure_entity: server_entity,
                                    serialized_chunk: cosmos_encoder::serialize(chunk),
                                }),
                            );
                        }
                    } else {
                        warn!("!!! Server received invalid entity from client {client_id}");
                    }
                }
                ClientReliableMessages::SendSingleChunk { structure_entity, chunk } => request_chunk_event_writer.send(RequestChunkEvent {
                    requester_id: client_id,
                    structure_entity,
                    chunk_coords: chunk,
                }),
                ClientReliableMessages::BreakBlock { structure_entity, block } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        break_block_event.send(BlockBreakEvent {
                            structure_entity,
                            breaker: player_entity,
                            structure_block: block,
                        });
                    }
                }
                ClientReliableMessages::PlaceBlock {
                    structure_entity,
                    block,
                    block_id,
                    block_up,
                    inventory_slot,
                } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        place_block_event.send(BlockPlaceEvent {
                            structure_entity,
                            structure_block: block,
                            block_id,
                            block_up,
                            inventory_slot: inventory_slot as usize,
                            placer: player_entity,
                        });
                    }
                }
                ClientReliableMessages::InteractWithBlock { structure_entity, block } => {
                    block_interact_event.send(BlockInteractEvent {
                        structure_entity,
                        structure_block: block,
                        interactor: lobby.player_from_id(client_id).unwrap(),
                    });
                }
                ClientReliableMessages::CreateShip { name: _name } => {
                    if let Some(client) = lobby.player_from_id(client_id) {
                        if let Ok((transform, location, looking, _)) = change_player_query.get(client) {
                            let ship_location =
                                *location + transform.rotation.mul_vec3(looking.rotation.mul_vec3(Vec3::new(0.0, 0.0, -4.0)));

                            create_ship_event_writer.send(CreateShipEvent {
                                ship_location,
                                rotation: looking.rotation,
                            });
                        }
                    }
                }
                ClientReliableMessages::PilotQuery { ship_entity } => {
                    let pilot = match pilot_query.get(ship_entity) {
                        Ok(pilot) => Some(pilot.entity),
                        _ => None,
                    };

                    server.send_message(
                        client_id,
                        NettyChannelServer::Reliable,
                        cosmos_encoder::serialize(&ServerReliableMessages::PilotChange {
                            structure_entity: ship_entity,
                            pilot_entity: pilot,
                        }),
                    );
                }
                ClientReliableMessages::StopPiloting => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        if let Ok(piloting) = pilot_query.get(player_entity) {
                            pilot_change_event_writer.send(ChangePilotEvent {
                                structure_entity: piloting.entity,
                                pilot_entity: None,
                            });
                        }
                    }
                }
                ClientReliableMessages::ChangeRenderDistance { mut render_distance } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        if let Some(mut e) = commands.get_entity(player_entity) {
                            if render_distance.sector_range > 8 {
                                render_distance.sector_range = 8;
                            }
                            e.insert(render_distance);
                        }
                    }
                }
                ClientReliableMessages::RequestEntityData { entity } => {
                    if commands.get_entity(entity).is_some() {
                        requested_entities_writer.send(RequestedEntityEvent { client_id, entity });
                    }
                }
                ClientReliableMessages::LeaveShip => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        if let Some(mut e) = commands.get_entity(player_entity) {
                            // This should be verified in the future to make sure the parent of the player is actually a ship
                            e.remove_parent();
                            if let Ok((player_trans, mut player_loc)) =
                                change_player_query.get_mut(player_entity).map(|(x, y, _, _)| (x, y))
                            {
                                player_loc.last_transform_loc = Some(player_trans.translation);
                            }

                            server.broadcast_message_except(
                                client_id,
                                NettyChannelServer::Reliable,
                                cosmos_encoder::serialize(&ServerReliableMessages::PlayerLeaveShip { player_entity }),
                            );
                        }
                    }
                }
                ClientReliableMessages::JoinShip { ship_entity } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        if let Some(mut e) = commands.get_entity(player_entity) {
                            // This should be verified in the future to make sure the entity is actually a ship
                            e.set_parent(ship_entity);

                            // This is stupid, but when a parent changes the transform isn't properly updated for the player, which is super cool.
                            // At some point this should be fixed in a way that doesn't require this stuck everywhere
                            if let Ok((mut trans, _, _, _)) = change_player_query.get_mut(player_entity) {
                                trans.translation -= non_player_transform_query
                                    .get(ship_entity)
                                    .expect("A ship should always have a transform.")
                                    .translation;
                            }

                            server.broadcast_message_except(
                                client_id,
                                NettyChannelServer::Reliable,
                                cosmos_encoder::serialize(&ServerReliableMessages::PlayerJoinShip {
                                    player_entity,
                                    ship_entity,
                                }),
                            );
                        }
                    }
                }
                ClientReliableMessages::ExitBuildMode => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        exit_build_mode_writer.send(ExitBuildModeEvent { player_entity });
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, server_listen_messages);
}
