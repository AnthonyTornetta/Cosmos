use bevy::{prelude::*, render::camera::Projection};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::blocks::Blocks,
    entities::player::Player,
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    netty::{
        client_reliable_messages::ClientReliableMessages, netty::*,
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages,
    },
    structure::{
        chunk::Chunk, events::ChunkSetEvent, planet::planet_builder::TPlanetBuilder,
        ship::ship_builder::TShipBuilder, structure::Structure,
    },
};

use crate::{
    camera::camera_controller::CameraHelper,
    netty::{
        flags::LocalPlayer,
        lobby::{ClientLobby, PlayerInfo},
        mapping::NetworkMapping,
    },
    state::game_state::GameState,
    structure::{
        planet::client_planet_builder::ClientPlanetBuilder,
        ship::client_ship_builder::ClientShipBuilder,
    },
};

fn client_sync_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut set_chunk_event_writer: EventWriter<ChunkSetEvent>,
    mut block_change_event_writer: EventWriter<BlockChangedEvent>,
    query_player: Query<&Player>,
    mut query_body: Query<(&mut Transform, &mut Velocity, Option<&LocalPlayer>)>,
    mut query_structure: Query<&mut Structure>,
    blocks: Res<Blocks>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(NettyChannel::Unreliable.id()) {
        let msg: ServerUnreliableMessages = bincode::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::PlayerBody { id, body } => {
                let entity = lobby.players.get(&id).unwrap().client_entity.clone();

                let (mut transform, mut velocity, _) = query_body.get_mut(entity).unwrap();

                transform.translation = body.translation.into();
                transform.rotation = body.rotation.into();

                velocity.linvel = body.body_vel.linvel.into();
                velocity.angvel = body.body_vel.angvel.into();
            }
            ServerUnreliableMessages::BulkBodies {
                bodies,
                time_stamp: _,
            } => {
                for (server_entity, body) in bodies.iter() {
                    let maybe_exists = network_mapping.client_from_server(&server_entity);

                    if maybe_exists.is_some() {
                        let entity = maybe_exists.unwrap();

                        let (mut transform, mut velocity, local) =
                            query_body.get_mut(*entity).unwrap();

                        if local.is_none() {
                            transform.translation = body.translation.into();
                            transform.rotation = body.rotation.into();

                            velocity.linvel = body.body_vel.linvel.into();
                            velocity.angvel = body.body_vel.angvel.into();
                        }
                    } else {
                        println!("Entity no exist!");
                    }
                }
            }
        }
    }

    while let Some(message) = client.receive_message(NettyChannel::Reliable.id()) {
        let msg: ServerReliableMessages = bincode::deserialize(&message).unwrap();

        match msg {
            ServerReliableMessages::PlayerCreate {
                body,
                id,
                entity,
                name,
            } => {
                println!("Player {} ({}) connected!", name.as_str(), id);

                let mut client_entity = commands.spawn();

                client_entity
                    .insert_bundle(PbrBundle {
                        transform: body.create_transform(),
                        mesh: meshes.add(shape::Capsule::default().into()),
                        ..default()
                    })
                    .insert(Collider::capsule_y(0.5, 0.25))
                    .insert(LockedAxes::ROTATION_LOCKED)
                    .insert(RigidBody::Dynamic)
                    .insert(body.create_velocity())
                    .insert(Player::new(name, id));

                if client_id == id {
                    client_entity
                        .insert(LocalPlayer::default())
                        .with_children(|parent| {
                            parent
                                .spawn_bundle(Camera3dBundle {
                                    transform: Transform::from_xyz(0.0, 0.75, 0.0),
                                    projection: Projection::from(PerspectiveProjection {
                                        fov: (90.0 / 360.0) * (std::f32::consts::PI * 2.0),
                                        ..default()
                                    }),
                                    ..default()
                                })
                                .insert(CameraHelper::default());
                        });
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity: client_entity.id(),
                };

                lobby.players.insert(id, player_info);
                network_mapping.add_mapping(&client_entity.id(), &entity);
            }
            ServerReliableMessages::PlayerRemove { id } => {
                if let Some(PlayerInfo {
                    client_entity,
                    server_entity,
                }) = lobby.players.remove(&id)
                {
                    let mut entity = commands.entity(client_entity);

                    let name = query_player.get(client_entity).unwrap().name.clone();
                    entity.despawn();
                    network_mapping.remove_mapping_from_server_entity(&server_entity);

                    println!("Player {} ({}) disconnected", name, id);
                }
            }
            ServerReliableMessages::PlanetCreate {
                entity: server_entity,
                length,
                height,
                width,
                body,
            } => {
                let mut entity = commands.spawn();
                let mut structure = Structure::new(width, height, length, entity.id());

                let builder = ClientPlanetBuilder::default();
                builder.insert_planet(&mut entity, body.create_transform(), &mut structure);

                entity.insert(structure);

                network_mapping.add_mapping(&entity.id(), &server_entity);

                // create_structure_writer.send(StructureCreated {
                //     entity: entity.id(),
                // });
            }
            ServerReliableMessages::ShipCreate {
                entity: server_entity,
                body,
                width,
                height,
                length,
            } => {
                let mut entity = commands.spawn();
                let mut structure = Structure::new(width, height, length, entity.id());

                let builder = ClientShipBuilder::default();
                builder.insert_ship(
                    &mut entity,
                    body.create_transform(),
                    body.create_velocity(),
                    &mut structure,
                );

                entity.insert(structure);

                network_mapping.add_mapping(&entity.id(), &server_entity);

                client.send_message(
                    NettyChannel::Reliable.id(),
                    bincode::serialize(&ClientReliableMessages::PilotQuery {
                        ship_entity: server_entity,
                    })
                    .unwrap(),
                );
            }
            ServerReliableMessages::ChunkData {
                structure_entity: server_structure_entity,
                serialized_chunk,
            } => {
                let s_entity = network_mapping
                    .client_from_server(&server_structure_entity)
                    .expect("Got chunk data for structure that doesn't exist on client");

                let mut structure = query_structure.get_mut(s_entity.clone()).unwrap();

                let chunk: Chunk = bincode::deserialize(&serialized_chunk).unwrap();

                let (x, y, z) = (
                    chunk.structure_x(),
                    chunk.structure_y(),
                    chunk.structure_z(),
                );

                structure.set_chunk(chunk);

                set_chunk_event_writer.send(ChunkSetEvent {
                    x,
                    y,
                    z,
                    structure_entity: s_entity.clone(),
                });
            }
            ServerReliableMessages::StructureRemove {
                entity: server_entity,
            } => {
                commands
                    .entity(
                        network_mapping
                            .client_from_server(&server_entity)
                            .unwrap()
                            .clone(),
                    )
                    .despawn_recursive();
            }
            ServerReliableMessages::MOTD { motd } => {
                println!("Server MOTD: {}", motd);
            }
            ServerReliableMessages::BlockChange {
                x,
                y,
                z,
                structure_entity,
                block_id,
            } => {
                let client_ent = network_mapping.client_from_server(&structure_entity);

                // Sometimes you'll get block updates for structures that don't exist
                if client_ent.is_some() {
                    let ent = client_ent.unwrap().clone();

                    let structure = query_structure.get_mut(ent);

                    if structure.is_ok() {
                        structure.unwrap().set_block_at(
                            x,
                            y,
                            z,
                            blocks.block_from_numeric_id(block_id),
                            &blocks,
                            Some(&mut block_change_event_writer),
                        );
                    } else {
                        println!("OH NO!");
                        commands.entity(ent.clone()).log_components();
                    }
                }
            }
            ServerReliableMessages::PilotChange {
                structure_entity,
                pilot_entity,
            } => {
                pilot_change_event_writer.send(ChangePilotEvent {
                    structure_entity: network_mapping
                        .client_from_server(&structure_entity)
                        .unwrap()
                        .clone(),
                    pilot_entity,
                });
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_update(GameState::LoadingWorld).with_system(client_sync_players),
    )
    .add_system_set(SystemSet::on_update(GameState::Playing).with_system(client_sync_players));
}
