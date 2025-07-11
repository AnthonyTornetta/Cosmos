//! Listens to almost all the messages received from the client
//!
//! Eventually this should be broken down into more specific functions

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::{ClientId, RenetServer};
use cosmos_core::block::block_events::{BlockBreakEvent, BlockInteractEvent, BlockPlaceEvent, BlockPlaceEventData};
use cosmos_core::ecs::mut_events::MutEvent;
use cosmos_core::entities::player::creative::Creative;
use cosmos_core::inventory::Inventory;
use cosmos_core::inventory::itemstack::ItemStackSystemSet;
use cosmos_core::item::Item;
use cosmos_core::netty::netty_rigidbody::NettyRigidBodyLocation;
use cosmos_core::netty::server::ServerLobby;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::netty::{NettyChannelClient, NettyChannelServer, cosmos_encoder};
use cosmos_core::physics::location::{Location, SetPosition};
use cosmos_core::registry::Registry;
use cosmos_core::state::GameState;
use cosmos_core::structure::loading::ChunksNeedLoaded;
use cosmos_core::structure::shared::build_mode::{BuildMode, ExitBuildModeEvent};
use cosmos_core::structure::systems::StructureSystems;
use cosmos_core::{
    entities::player::Player,
    events::structure::change_pilot_event::ChangePilotEvent,
    netty::{
        client_reliable_messages::ClientReliableMessages, client_unreliable_messages::ClientUnreliableMessages,
        server_reliable_messages::ServerReliableMessages,
    },
    structure::{Structure, ship::pilot::Pilot},
};

use crate::entities::player::PlayerLooking;
use crate::structure::planet::chunk::ChunkNeedsSent;
use crate::structure::planet::generation::planet_generator::RequestChunkEvent;
use crate::structure::ship::events::{CreateShipEvent, ShipSetMovementEvent};
use crate::structure::station::events::CreateStationEvent;

use super::server_events::handle_server_events;

#[derive(Resource, Default)]
struct SendAllChunks(HashMap<Entity, Vec<ClientId>>);

/// Bevy system that listens to almost all the messages received from the client
///
/// Eventually this should be broken down into more specific functions
fn server_listen_messages(
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
        mut create_station_event_writer,
        mut request_chunk_event_writer,
    ): (
        Query<&mut StructureSystems>,
        EventWriter<BlockBreakEvent>,
        EventWriter<MutEvent<BlockPlaceEvent>>,
        EventWriter<BlockInteractEvent>,
        EventWriter<ExitBuildModeEvent>,
        EventWriter<CreateShipEvent>,
        EventWriter<CreateStationEvent>,
        EventWriter<RequestChunkEvent>,
    ),
    mut q_inventory: Query<&mut Inventory>,
    q_creative: Query<(), With<Creative>>,
    items: Res<Registry<Item>>,
    (mut ship_movement_event_writer, mut pilot_change_event_writer): (EventWriter<ShipSetMovementEvent>, EventWriter<ChangePilotEvent>),
    pilot_query: Query<&Pilot>,
    player_parent_location: Query<&Location, Without<Player>>,
    mut q_player: Query<(&GlobalTransform, &mut Transform, &mut Location, &mut PlayerLooking, &mut Velocity), With<Player>>,
    mut build_mode: Query<&mut BuildMode>,

    mut send_all_chunks: ResMut<SendAllChunks>,
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
                        if let Ok((_, mut transform, mut location, mut currently_looking, mut velocity)) = q_player.get_mut(player_entity) {
                            match body.location {
                                NettyRigidBodyLocation::Absolute(new_location) => {
                                    commands.entity(player_entity).insert(SetPosition::Transform);

                                    *location = new_location;
                                }
                                NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                                    if !player_parent_location.contains(entity) {
                                        // Probably out of date info, just ignore
                                        continue;
                                    };
                                    commands.entity(player_entity).insert(SetPosition::Location);

                                    transform.translation = rel_trans;
                                }
                            };

                            currently_looking.rotation = looking;
                            velocity.linvel = body.body_vel.map(|x| x.linvel).unwrap_or(Vec3::ZERO);
                            transform.rotation = body.rotation;
                        }
                    }
                    ClientUnreliableMessages::SetMovement { movement } => {
                        if let Ok(pilot) = pilot_query.get(player_entity) {
                            let ship = pilot.entity;

                            ship_movement_event_writer.write(ShipSetMovementEvent { movement, ship });
                        }
                    }
                    ClientUnreliableMessages::ShipActiveSystem(active_system) => {
                        if let Ok(pilot) = pilot_query.get(player_entity)
                            && let Ok(mut systems) = systems_query.get_mut(pilot.entity)
                        {
                            systems.set_active_system(active_system, &mut commands);
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
                    let Ok(structure) = structure_query.get(server_entity) else {
                        warn!("!!! Server received invalid entity from client {client_id}; entity = {server_entity:?}");
                        continue;
                    };

                    let Structure::Full(_) = structure else {
                        warn!("Cannot request all chunks for a dynamic structure! Requester: {client_id}; entity = {server_entity:?}");
                        continue;
                    };

                    info!("Send all chunks for received {server_entity:?}!");

                    send_all_chunks.0.entry(server_entity).or_default().push(client_id);
                }
                ClientReliableMessages::SendSingleChunk { structure_entity, chunk } => {
                    request_chunk_event_writer.write(RequestChunkEvent {
                        requester_id: client_id,
                        structure_entity,
                        chunk_coords: chunk,
                    });
                }
                ClientReliableMessages::BreakBlock { block } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        break_block_event.write(BlockBreakEvent {
                            breaker: player_entity,
                            block,
                        });
                    }
                }
                ClientReliableMessages::PlaceBlock {
                    block,
                    block_id,
                    block_rotation: block_up,
                    inventory_slot,
                } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        place_block_event.write(
                            BlockPlaceEvent::Event(BlockPlaceEventData {
                                structure_block: block,
                                block_id,
                                block_up,
                                inventory_slot: inventory_slot as usize,
                                placer: player_entity,
                            })
                            .into(),
                        );
                    }
                }
                ClientReliableMessages::InteractWithBlock {
                    block,
                    block_including_fluids,
                    alternate,
                } => {
                    block_interact_event.write(BlockInteractEvent {
                        block,
                        block_including_fluids,
                        interactor: lobby.player_from_id(client_id).unwrap(),
                        alternate,
                    });
                }
                ClientReliableMessages::CreateShip { name } => {
                    let Some(client) = lobby.player_from_id(client_id) else {
                        warn!("Invalid client id {client_id}");
                        continue;
                    };

                    if !q_creative.contains(client) {
                        let Ok(mut inventory) = q_inventory.get_mut(client) else {
                            info!("No inventory ;(");
                            continue;
                        };

                        let Some(ship_core) = items.from_id("cosmos:ship_core") else {
                            info!("Does not have ship core registered");
                            continue;
                        };

                        let (remaining_didnt_take, _) = inventory.take_and_remove_item(ship_core, 1, &mut commands);
                        if remaining_didnt_take != 0 {
                            info!("Does not have ship core");
                            continue;
                        }
                    }

                    if let Ok((g_trans, _, location, looking, _)) = q_player.get(client) {
                        let rotation = g_trans.rotation() * looking.rotation;

                        let ship_location = *location + rotation * Vec3::new(0.0, 0.0, -4.0);

                        info!("Creating ship {name}");

                        create_ship_event_writer.write(CreateShipEvent {
                            ship_location,
                            rotation,
                            creator: client,
                        });
                    } else {
                        warn!("Invalid player entity - {client:?}");
                    }
                }
                ClientReliableMessages::CreateStation { name } => {
                    let Some(client) = lobby.player_from_id(client_id) else {
                        warn!("Invalid client id {client_id}");
                        continue;
                    };

                    if !q_creative.contains(client) {
                        let Ok(mut inventory) = q_inventory.get_mut(client) else {
                            info!("No inventory ;(");
                            continue;
                        };

                        let Some(station_core) = items.from_id("cosmos:station_core") else {
                            info!("Does not have station core registered");
                            continue;
                        };

                        let (remaining_didnt_take, _) = inventory.take_and_remove_item(station_core, 1, &mut commands);
                        if remaining_didnt_take != 0 {
                            info!("Does not have station core");
                            continue;
                        }
                    }

                    if let Ok((g_trans, _, location, looking, _)) = q_player.get(client) {
                        let rotation = g_trans.rotation() * looking.rotation;

                        let station_location = *location + rotation * Vec3::new(0.0, 0.0, -4.0);

                        info!("Creating ship {name}");

                        create_station_event_writer.write(CreateStationEvent {
                            station_location,
                            rotation,
                        });
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
                    if let Some(player_entity) = lobby.player_from_id(client_id)
                        && let Ok(piloting) = pilot_query.get(player_entity)
                    {
                        pilot_change_event_writer.write(ChangePilotEvent {
                            structure_entity: piloting.entity,
                            pilot_entity: None,
                        });
                    }
                }
                ClientReliableMessages::ChangeRenderDistance { mut render_distance } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id)
                        && let Ok(mut e) = commands.get_entity(player_entity)
                    {
                        if render_distance.sector_range > 8 {
                            render_distance.sector_range = 8;
                        }
                        e.insert(render_distance);
                    }
                }
                ClientReliableMessages::LeaveShip => {
                    if let Some(player_entity) = lobby.player_from_id(client_id)
                        && let Ok(mut e) = commands.get_entity(player_entity)
                    {
                        // This should be verified in the future to make sure the parent of the player is actually a ship
                        e.remove_parent_in_place();
                        // if let Ok((player_trans, mut player_loc)) =
                        //     change_player_query.get_mut(player_entity).map(|(x, y, _, _)| (x, y))
                        // {
                        //     player_loc.last_transform_loc = Some(player_trans.translation);
                        // }

                        server.broadcast_message_except(
                            client_id,
                            NettyChannelServer::Reliable,
                            cosmos_encoder::serialize(&ServerReliableMessages::PlayerLeaveShip { player_entity }),
                        );
                    }
                }
                ClientReliableMessages::ExitBuildMode => {
                    if let Some(player_entity) = lobby.player_from_id(client_id) {
                        exit_build_mode_writer.write(ExitBuildModeEvent { player_entity });
                    }
                }
                ClientReliableMessages::SetSymmetry { axis, coordinate } => {
                    if let Some(player_entity) = lobby.player_from_id(client_id)
                        && let Ok(mut build_mode) = build_mode.get_mut(player_entity)
                    {
                        if let Some(coordinate) = coordinate {
                            build_mode.set_symmetry(axis, coordinate);
                        } else {
                            build_mode.remove_symmetry(axis);
                        }
                    }
                }
            }
        }
    }
}

fn send_all_chunks(
    mut send_all_chunks: ResMut<SendAllChunks>,
    q_structure: Query<&Structure>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
) {
    send_all_chunks.0.retain(|&structure_entity, client_ids| {
        let Ok(structure) = q_structure.get(structure_entity) else {
            return false;
        };

        let Structure::Full(structure) = structure else {
            panic!("Verified in `server_listen_messages`");
        };

        if !structure.is_loaded() {
            return true;
        }

        let message = cosmos_encoder::serialize(&ServerReliableMessages::NumberOfChunks {
            entity: structure_entity,
            chunks_needed: ChunksNeedLoaded {
                amount_needed: structure.chunks().len(),
            },
        });

        for &client_id in client_ids.iter() {
            server.send_message(client_id, NettyChannelServer::Reliable, message.clone());
        }

        info!("Sending chunks for {structure_entity:?}!");

        for (_, chunk) in structure.chunks() {
            let Some(entity) = structure.chunk_entity(chunk.chunk_coordinates()) else {
                error!("Missing chunk entity in entity {structure_entity:?} - logging components!");
                commands.entity(structure_entity).log_components();
                return true;
            };

            commands.entity(entity).insert(ChunkNeedsSent {
                client_ids: client_ids.clone(),
            });
        }

        false
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (handle_server_events, server_listen_messages)
            .chain()
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .before(ItemStackSystemSet::CreateDataEntity)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, send_all_chunks.in_set(NetworkingSystemsSet::SyncComponents))
    .init_resource::<SendAllChunks>();
}
