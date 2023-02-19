use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::physics::location::{bubble_down_locations, Location};
use cosmos_core::physics::player_world::{PlayerWorld, WorldWithin};
use cosmos_core::structure::systems::{SystemActive, Systems};
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
        {structure_block::StructureBlock, Structure},
    },
};

use crate::entities::player::PlayerLooking;
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
    structure_query: Query<&Structure>,
    mut systems_query: Query<&mut Systems>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
    mut create_ship_event_writer: EventWriter<CreateShipEvent>,

    mut ship_movement_event_writer: EventWriter<ShipSetMovementEvent>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
    pilot_query: Query<&Pilot>,
    mut change_player_query: Query<
        (&Transform, &mut Location, &mut PlayerLooking, &mut Velocity),
        With<Player>,
    >,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannel::Unreliable.id()) {
            if let Some(player_entity) = lobby.players.get(&client_id) {
                let command: ClientUnreliableMessages = bincode::deserialize(&message).unwrap();

                match command {
                    ClientUnreliableMessages::PlayerBody { body, looking } => {
                        if let Ok((transform, mut location, mut currently_looking, mut velocity)) =
                            change_player_query.get_mut(*player_entity)
                        {
                            location.set_from(&body.location);
                            location.last_transform_loc = transform.translation;
                            currently_looking.rotation = looking;
                            velocity.linvel = body.body_vel.linvel.into();
                        }
                    }
                    ClientUnreliableMessages::SetMovement { movement } => {
                        if let Ok(pilot) = pilot_query.get(*player_entity) {
                            let ship = pilot.entity;

                            ship_movement_event_writer
                                .send(ShipSetMovementEvent { movement, ship });
                        }
                    }
                    ClientUnreliableMessages::ShipStatus { use_system } => {
                        if let Ok(pilot) = pilot_query.get(*player_entity) {
                            if use_system {
                                commands.entity(pilot.entity).insert(SystemActive);
                            } else {
                                commands.entity(pilot.entity).remove::<SystemActive>();
                            }
                        }
                    }
                    ClientUnreliableMessages::ShipActiveSystem { active_system } => {
                        if let Ok(pilot) = pilot_query.get(*player_entity) {
                            if let Ok(mut systems) = systems_query.get_mut(pilot.entity) {
                                systems.set_active_system(active_system, &mut commands);
                            }
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
                        println!("!!! Server received invalid entity from client {client_id}");
                    }
                }
                ClientReliableMessages::BreakBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        break_block_event.send(BlockBreakEvent {
                            structure_entity,
                            breaker: *player_entity,
                            x: x as usize,
                            y: y as usize,
                            z: z as usize,
                        });
                    }
                }
                ClientReliableMessages::PlaceBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                    block_id,
                    inventory_slot,
                } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        place_block_event.send(BlockPlaceEvent {
                            structure_entity,
                            x: x as usize,
                            y: y as usize,
                            z: z as usize,
                            block_id,
                            inventory_slot: inventory_slot as usize,
                            placer: *player_entity,
                        });
                    }
                }
                ClientReliableMessages::InteractWithBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                } => {
                    block_interact_event.send(BlockInteractEvent {
                        structure_entity,
                        structure_block: StructureBlock::new(x as usize, y as usize, z as usize),
                        interactor: *lobby.players.get(&client_id).unwrap(),
                    });
                }
                ClientReliableMessages::CreateShip { name: _name } => {
                    if let Some(client) = lobby.players.get(&client_id) {
                        let (_, location, looking, _) = change_player_query.get(*client).unwrap();

                        let ship_location =
                            *location + looking.rotation.mul_vec3(Vec3::new(0.0, 0.0, -4.0));

                        create_ship_event_writer.send(CreateShipEvent {
                            ship_location,
                            rotation: looking.rotation,
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

// fn sync_locations(mut query: Query<(&Transform, &mut Location), Changed<Transform>>) {
//     for (trans, mut loc) in query.iter_mut() {
//         // Really not that great, but I can't think of any other way of avoiding recursively changing each other
//         loc.bypass_change_detection().local = trans.translation;
//     }
// }

// /// This has to be put after specific systems in the server/client or jitter happens
// fn sync_translations(mut query: Query<(&mut Transform, &Location), Changed<Location>>) {
//     for (mut trans, loc) in query.iter_mut() {
//         // Really not that great, but I can't think of any other way of avoiding recursively changing each other
//         trans.bypass_change_detection().translation = loc.local;
//     }
// }

fn sync_transforms_and_locations(
    mut trans_query_no_parent: Query<
        (Entity, &mut Transform, &mut Location, &WorldWithin),
        (Without<PlayerWorld>, Without<Parent>),
    >,
    mut trans_query_with_parent: Query<
        (Entity, &mut Transform, &mut Location),
        (Without<PlayerWorld>, With<Parent>),
    >,
    is_player: Query<(), With<Player>>,
    parent_query: Query<&Parent>,
    entity_query: Query<Entity>,
    mut world_query: Query<(Entity, &PlayerWorld, &mut Location)>,
) {
    // println!("=========== START ==============");
    // for (entity, transform, _, _) in trans_query_no_parent.iter() {
    //     if is_player.contains(entity) {
    //         println!("{}", transform.translation);
    //     }
    // }

    for (entity, transform, mut location, _) in trans_query_no_parent.iter_mut() {
        // Server transforms for players should NOT be applied to the location.
        // The location the client sent should override it.
        if !is_player.contains(entity) {
            location.apply_updates(transform.translation);
        }
    }
    for (entity, transform, mut location) in trans_query_with_parent.iter_mut() {
        // Server transforms for players should NOT be applied to the location.
        // The location the client sent should override it.
        if !is_player.contains(entity) {
            location.apply_updates(transform.translation);
        }
    }

    for (world_entity, world, mut world_location) in world_query.iter_mut() {
        let mut player_entity = entity_query
            .get(world.player)
            .expect("This player should exist, but will break when someone disconnects.");

        while let Ok(parent) = parent_query.get(player_entity) {
            let parent_entity = parent.get();
            if trans_query_no_parent.contains(parent_entity) {
                player_entity = parent.get();
            } else {
                break;
            }
        }

        let location = trans_query_no_parent
            .get(player_entity)
            .map(|x| x.2)
            .or_else(|_| match trans_query_with_parent.get(player_entity) {
                Ok((_, _, loc)) => Ok(loc),
                Err(x) => Err(x),
            })
            .expect("The above loop guarantees this is valid");

        world_location.set_from(&location);

        // println!("Player loc: {location}");

        // Update transforms of objects within this world.
        for (_, mut transform, mut location, world_within) in trans_query_no_parent.iter_mut() {
            if world_within.0 == world_entity {
                // if entity.index() != 0 {
                //     println!("Translating: {}", transform.translation);
                // }

                transform.translation = world_location.relative_coords_to(&location);
                location.last_transform_loc = transform.translation;

                // if entity.index() != 0 {
                //     println!("Final transform: {}", transform.translation);
                // }
            }
        }

        // let (mut player_transform, mut player_location) = player_trans_query.single_mut();

        // player_location.apply_updates(player_transform.translation);

        // world_location.set_from(&player_location);

        // player_transform.translation = world_location.relative_coords_to(&player_location);
        // player_location.last_transform_loc = player_transform.translation;

        // for (mut transform, mut location) in trans_query.iter_mut() {
        //     location.apply_updates(transform.translation);
        //     transform.translation = world_location.relative_coords_to(&location);
        //     location.last_transform_loc = transform.translation;
        // }
    }
}

pub fn register(app: &mut App) {
    app.add_system(server_listen_messages)
        // If it's not after this system, some noticable jitter can happen
        .add_system(sync_transforms_and_locations.after(server_listen_messages))
        .add_system(bubble_down_locations.after(sync_transforms_and_locations));
}
