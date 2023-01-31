use bevy::{core_pipeline::bloom::BloomSettings, prelude::*, render::camera::Projection};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::Block,
    entities::player::Player,
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    inventory::Inventory,
    netty::{
        client_reliable_messages::ClientReliableMessages,
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages, NettyChannel,
    },
    projectiles::laser::Laser,
    registry::Registry,
    structure::{
        chunk::Chunk,
        events::ChunkSetEvent,
        planet::planet_builder::TPlanetBuilder,
        ship::{pilot::Pilot, ship_builder::TShipBuilder, Ship},
        Structure,
    },
};

use crate::{
    camera::camera_controller::CameraHelper,
    events::ship::set_ship_event::SetShipMovementEvent,
    netty::{
        flags::LocalPlayer,
        lobby::{ClientLobby, PlayerInfo},
        mapping::NetworkMapping,
    },
    state::game_state::GameState,
    structure::{
        chunk_retreiver::NeedsPopulated, planet::client_planet_builder::ClientPlanetBuilder,
        ship::client_ship_builder::ClientShipBuilder,
    },
    ui::crosshair::CrosshairOffset,
};

#[derive(Component)]
struct LastRotation(Quat);

fn insert_last_rotation(mut commands: Commands, query: Query<Entity, Added<Structure>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(LastRotation(Quat::IDENTITY));
    }
}

fn update_crosshair(
    mut query: Query<(&Pilot, &mut LastRotation, &Transform), (With<Ship>, Changed<Transform>)>,
    local_player_query: Query<Entity, With<LocalPlayer>>,
    camera_query: Query<(Entity, &Camera)>,
    transform_query: Query<&GlobalTransform>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    windows: Res<Windows>,
) {
    for (pilot, mut last_rotation, transform) in query.iter_mut() {
        if local_player_query.get(pilot.entity).is_ok() {
            // let (cam, global) = cam_query.get_single().unwrap();

            let (cam_entity, camera) = camera_query.get_single().unwrap();

            let cam_global = transform_query.get(cam_entity).unwrap();

            let primary = windows.get_primary().unwrap();

            if let Some(mut pos_on_screen) = camera.world_to_viewport(
                cam_global,
                last_rotation.0.mul_vec3(Vec3::new(0.0, 0.0, -1.0)) + cam_global.translation(),
            ) {
                pos_on_screen -= Vec2::new(primary.width() / 2.0, primary.height() / 2.0);

                crosshair_offset.x += pos_on_screen.x;
                crosshair_offset.y += pos_on_screen.y;
            }

            last_rotation.0 = transform.rotation;
        }
    }
}

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
    blocks: Res<Registry<Block>>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
    mut set_ship_movement_event: EventWriter<SetShipMovementEvent>,
    time: Res<Time>,

    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(NettyChannel::Unreliable.id()) {
        let msg: ServerUnreliableMessages = bincode::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::PlayerBody { id, body } => {
                if let Some(entity) = lobby
                    .players
                    .get(&id)
                    .map_or(None, |x| Some(x.client_entity))
                {
                    let (mut transform, mut velocity, _) = query_body.get_mut(entity).unwrap();

                    transform.translation = body.translation.into();
                    transform.rotation = body.rotation.into();

                    velocity.linvel = body.body_vel.linvel.into();
                    velocity.angvel = body.body_vel.angvel.into();
                }
            }
            ServerUnreliableMessages::BulkBodies {
                bodies,
                time_stamp: _,
            } => {
                for (server_entity, body) in bodies.iter() {
                    if let Some(entity) = network_mapping.client_from_server(server_entity) {
                        if let Ok((mut transform, mut velocity, local)) =
                            query_body.get_mut(*entity)
                        {
                            if local.is_none() {
                                transform.translation = body.translation.into();
                                transform.rotation = body.rotation.into();

                                velocity.linvel = body.body_vel.linvel.into();
                                velocity.angvel = body.body_vel.angvel.into();
                            }
                        }
                    }
                }
            }
            ServerUnreliableMessages::SetMovement {
                movement,
                ship_entity,
            } => {
                set_ship_movement_event.send(SetShipMovementEvent {
                    ship_entity,
                    ship_movement: movement,
                });
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
                inventory_serialized,
            } => {
                println!("Player {} ({}) connected!", name.as_str(), id);

                let mut client_entity = commands.spawn_empty();

                let inventory: Inventory = bincode::deserialize(&inventory_serialized).unwrap();

                client_entity
                    .insert(PbrBundle {
                        transform: body.create_transform(),
                        mesh: meshes.add(shape::Capsule::default().into()),
                        ..default()
                    })
                    .insert(Collider::capsule_y(0.5, 0.25))
                    .insert(LockedAxes::ROTATION_LOCKED)
                    .insert(RigidBody::Dynamic)
                    .insert(body.create_velocity())
                    .insert(Player::new(name, id))
                    .insert(ReadMassProperties::default())
                    .insert(inventory);

                if client_id == id {
                    client_entity
                        .insert(LocalPlayer::default())
                        .with_children(|parent| {
                            parent
                                .spawn(Camera3dBundle {
                                    camera: Camera {
                                        hdr: true,
                                        ..Default::default()
                                    },
                                    transform: Transform::from_xyz(0.0, 0.75, 0.0),
                                    projection: Projection::from(PerspectiveProjection {
                                        fov: (90.0 / 360.0) * (std::f32::consts::PI * 2.0),
                                        ..default()
                                    }),
                                    ..default()
                                })
                                .insert(BloomSettings {
                                    ..Default::default()
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

                    println!("Player {name} ({id}) disconnected");
                }
            }
            ServerReliableMessages::PlanetCreate {
                entity: server_entity,
                length,
                height,
                width,
                body,
            } => {
                let mut entity = commands.spawn_empty();
                let mut structure = Structure::new(
                    width as usize,
                    height as usize,
                    length as usize,
                    entity.id(),
                );

                let builder = ClientPlanetBuilder::default();
                builder.insert_planet(&mut entity, body.create_transform(), &mut structure);

                entity.insert(structure).insert(NeedsPopulated);

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
                let mut entity = commands.spawn_empty();
                let mut structure = Structure::new(
                    width as usize,
                    height as usize,
                    length as usize,
                    entity.id(),
                );

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

                let mut structure = query_structure.get_mut(*s_entity).unwrap();

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
                    structure_entity: *s_entity,
                });
            }
            ServerReliableMessages::StructureRemove {
                entity: server_entity,
            } => {
                if let Some(entity) = network_mapping.client_from_server(&server_entity) {
                    commands.entity(*entity).despawn_recursive();
                }
            }
            ServerReliableMessages::MOTD { motd } => {
                println!("Server MOTD: {motd}");
            }
            ServerReliableMessages::BlockChange {
                x,
                y,
                z,
                structure_entity,
                block_id,
            } => {
                // Sometimes you'll get block updates for structures that don't exist
                if let Some(client_ent) = network_mapping.client_from_server(&structure_entity) {
                    if let Ok(mut structure) = query_structure.get_mut(*client_ent) {
                        structure.set_block_at(
                            x as usize,
                            y as usize,
                            z as usize,
                            blocks.from_numeric_id(block_id),
                            &blocks,
                            Some(&mut block_change_event_writer),
                        );
                    } else {
                        println!("OH NO!");
                        commands.entity(*client_ent).log_components();
                    }
                }
            }
            ServerReliableMessages::PilotChange {
                structure_entity,
                pilot_entity,
            } => {
                let entity = if let Some(pilot_entity) = pilot_entity {
                    network_mapping.client_from_server(&pilot_entity).copied()
                } else {
                    None
                };

                pilot_change_event_writer.send(ChangePilotEvent {
                    structure_entity: *network_mapping
                        .client_from_server(&structure_entity)
                        .unwrap(),
                    pilot_entity: entity,
                });
            }
            ServerReliableMessages::EntityInventory {
                serialized_inventory,
                owner,
            } => {
                if let Some(client_entity) = network_mapping.client_from_server(&owner) {
                    let inventory: Inventory = bincode::deserialize(&serialized_inventory).unwrap();

                    commands.entity(*client_entity).insert(inventory);
                } else {
                    eprintln!(
                        "Error: unrecognized entity {} received from server!",
                        owner.index()
                    );
                }
            }
            ServerReliableMessages::LaserCannonFire {} => {
                println!("A laser cannon was fired")
            }
            ServerReliableMessages::CreateLaser {
                color,
                position,
                laser_velocity,
                firer_velocity,
                strength,
                mut no_hit,
            } => {
                if let Some(server_entity) = no_hit {
                    if let Some(client_entity) = network_mapping.client_from_server(&server_entity)
                    {
                        no_hit = Some(*client_entity);
                    }
                }

                // let laser_entity =
                Laser::spawn_custom_pbr(
                    position,
                    laser_velocity,
                    firer_velocity,
                    strength,
                    no_hit,
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Box::new(0.1, 0.1, 1.0))),
                        material: materials.add(StandardMaterial {
                            base_color: color,
                            emissive: color,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    &time,
                    &mut commands,
                );

                // too laggy (and strobey) ;(
                // commands.entity(laser_entity).with_children(|parent| {
                //     parent.spawn(PointLightBundle {
                //         transform: Transform::from_xyz(0.0, 0.0, 0.0),
                //         point_light: PointLight {
                //             intensity: 100.0,
                //             range: 10.0,
                //             color,
                //             shadows_enabled: false,
                //             ..default()
                //         },
                //         ..default()
                //     });
                // });
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_update(GameState::LoadingWorld).with_system(client_sync_players),
    )
    .add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(client_sync_players)
            .with_system(update_crosshair)
            .with_system(insert_last_rotation),
    );
}
