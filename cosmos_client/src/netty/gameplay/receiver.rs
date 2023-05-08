//! A receiver + processor for a bunch of network packets.
//!
//! This should eventually be broken up

use bevy::{
    core_pipeline::bloom::BloomSettings, prelude::*, render::camera::Projection,
    window::PrimaryWindow,
};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::Block,
    entities::player::{render_distance::RenderDistance, Player},
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    inventory::Inventory,
    netty::{
        client_reliable_messages::ClientReliableMessages, cosmos_encoder,
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages, NettyChannel,
    },
    persistence::LoadingDistance,
    physics::{
        location::{bubble_down_locations, Location, SYSTEM_SECTORS},
        player_world::PlayerWorld,
    },
    registry::Registry,
    structure::{
        chunk::Chunk,
        planet::{biosphere::BiosphereMarker, planet_builder::TPlanetBuilder},
        ship::{pilot::Pilot, ship_builder::TShipBuilder, Ship},
        ChunkInitEvent, Structure,
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
    rendering::MainCamera,
    state::game_state::GameState,
    structure::{
        planet::client_planet_builder::ClientPlanetBuilder,
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
    camera_query: Query<(Entity, &Camera), With<MainCamera>>,
    transform_query: Query<&GlobalTransform>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    for (pilot, mut last_rotation, transform) in query.iter_mut() {
        if local_player_query.get(pilot.entity).is_ok() {
            // let (cam, global) = cam_query.get_single().unwrap();

            let (cam_entity, camera) = camera_query.get_single().unwrap();

            let cam_global = transform_query.get(cam_entity).unwrap();

            let primary = primary_query.get_single().expect("Missing primary window");

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

#[derive(Resource, Debug, Default)]
struct RequestedEntities {
    entities: Vec<(Entity, f32)>,
}

fn client_sync_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut set_chunk_event_writer: EventWriter<ChunkInitEvent>,
    mut block_change_event_writer: EventWriter<BlockChangedEvent>,
    query_player: Query<&Player>,
    mut query_body: Query<
        (
            Option<&mut Location>,
            Option<&mut Transform>,
            Option<&mut Velocity>,
        ),
        Without<LocalPlayer>,
    >,
    world_center: Query<&Location, With<LocalPlayer>>,
    mut query_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
    mut set_ship_movement_event: EventWriter<SetShipMovementEvent>,
    mut requested_entities: ResMut<RequestedEntities>,
    time: Res<Time>,
) {
    let client_id = client.client_id();

    let mut new_entities = Vec::with_capacity(requested_entities.entities.len());

    for ent in requested_entities.entities.iter_mut() {
        ent.1 += time.delta_seconds();
        if ent.1 < 10.0 {
            new_entities.push(*ent);
        }
    }

    requested_entities.entities = new_entities;

    while let Some(message) = client.receive_message(NettyChannel::Unreliable.id()) {
        let msg: ServerUnreliableMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::BulkBodies {
                bodies,
                time_stamp: _,
            } => {
                for (server_entity, body) in bodies.iter() {
                    if let Some(entity) = network_mapping.client_from_server(server_entity) {
                        if let Ok((location, transform, velocity)) = query_body.get_mut(entity) {
                            match (location, transform, velocity) {
                                (Some(mut location), Some(mut transform), Some(mut velocity)) => {
                                    location.set_from(&body.location);

                                    transform.rotation = body.rotation;

                                    velocity.linvel = body.body_vel.linvel.into();
                                    velocity.angvel = body.body_vel.angvel.into();
                                }
                                _ => {
                                    let world_center = world_center.get_single().expect("There should only ever be one local player, and they should always exist.");

                                    let transform = body.create_transform(world_center);

                                    let mut location = body.location;
                                    location.last_transform_loc = Some(transform.translation);

                                    commands.entity(entity).insert((
                                        location,
                                        TransformBundle::from_transform(transform),
                                        body.create_velocity(),
                                    ));
                                }
                            }
                        }
                    } else if !requested_entities
                        .entities
                        .iter()
                        .any(|x| x.0 == *server_entity)
                    {
                        requested_entities.entities.push((*server_entity, 0.0));

                        client.send_message(
                            NettyChannel::Reliable.id(),
                            cosmos_encoder::serialize(&ClientReliableMessages::RequestEntityData {
                                entity: *server_entity,
                            }),
                        );
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
        let msg: ServerReliableMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerReliableMessages::PlayerCreate {
                mut body,
                id,
                entity: server_entity,
                name,
                inventory_serialized,
                render_distance: _,
            } => {
                // Prevents creation of duplicate players
                if lobby.players.contains_key(&id) {
                    println!("WARNING - DUPLICATE PLAYER RECEIVED {id}");
                    break;
                }

                println!("Player {} ({}) connected!", name.as_str(), id);

                let mut entity_cmds = commands.spawn_empty();

                let inventory: Inventory =
                    cosmos_encoder::deserialize(&inventory_serialized).unwrap();

                // This should be set via the server, but just in case,
                // this will avoid any position mismatching
                body.location.last_transform_loc = Some(body.location.local);

                entity_cmds.insert((
                    PbrBundle {
                        transform: Transform::with_rotation(
                            Transform::from_translation(body.location.local),
                            body.rotation,
                        ),
                        mesh: meshes.add(shape::Capsule::default().into()),
                        ..default()
                    },
                    body.location,
                    Collider::capsule_y(0.5, 0.25),
                    LockedAxes::ROTATION_LOCKED,
                    RigidBody::Dynamic,
                    body.create_velocity(),
                    Player::new(name, id),
                    ReadMassProperties::default(),
                    inventory,
                ));

                let client_entity = entity_cmds.id();

                let player_info = PlayerInfo {
                    server_entity,
                    client_entity,
                };

                lobby.players.insert(id, player_info);
                network_mapping.add_mapping(client_entity, server_entity);

                if client_id == id {
                    entity_cmds
                        .insert(LocalPlayer::default())
                        .insert(RenderDistance::default())
                        .with_children(|parent| {
                            parent.spawn((
                                Camera3dBundle {
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
                                },
                                BloomSettings {
                                    ..Default::default()
                                },
                                CameraHelper::default(),
                                MainCamera,
                                // No double UI rendering
                                UiCameraConfig { show_ui: false },
                            ));
                        });

                    commands.spawn((
                        PlayerWorld {
                            player: client_entity,
                        },
                        body.location,
                        PhysicsWorld {
                            world_id: DEFAULT_WORLD_ID,
                        },
                    ));
                }
            }
            ServerReliableMessages::PlayerRemove { id } => {
                if let Some(PlayerInfo {
                    client_entity,
                    server_entity,
                }) = lobby.players.remove(&id)
                {
                    if let Some(entity) = commands.get_entity(client_entity) {
                        if let Ok(player) = query_player.get(client_entity) {
                            println!("Player {} ({id}) disconnected", player.name());
                        }

                        entity.despawn_recursive();
                    }

                    network_mapping.remove_mapping_from_server_entity(&server_entity);
                }
            }
            ServerReliableMessages::Planet {
                entity: server_entity,
                length,
                height,
                width,
                body,
                planet,
                biosphere,
            } => {
                if network_mapping.contains_server_entity(server_entity) {
                    println!("Got duplicate planet! Is the server lagging?");
                    break;
                }

                let mut entity_cmds = commands.spawn_empty();
                let mut structure =
                    Structure::new(width as usize, height as usize, length as usize);

                let builder = ClientPlanetBuilder::default();
                builder.insert_planet(&mut entity_cmds, body.location, &mut structure, planet);

                entity_cmds.insert((structure, BiosphereMarker::new(biosphere)));

                let entity = entity_cmds.id();

                network_mapping.add_mapping(entity, server_entity);
            }
            ServerReliableMessages::Ship {
                entity: server_entity,
                body,
                width,
                height,
                length,
                chunks_needed,
            } => {
                if network_mapping.contains_server_entity(server_entity) {
                    println!("Got duplicate ship! Is the server lagging?");
                    break;
                }

                let mut entity_cmds = commands.spawn_empty();
                let mut structure =
                    Structure::new(width as usize, height as usize, length as usize);

                let builder = ClientShipBuilder::default();
                builder.insert_ship(
                    &mut entity_cmds,
                    body.location,
                    body.create_velocity(),
                    &mut structure,
                );

                entity_cmds.insert((structure, chunks_needed));

                let entity = entity_cmds.id();

                network_mapping.add_mapping(entity, server_entity);

                client.send_message(
                    NettyChannel::Reliable.id(),
                    cosmos_encoder::serialize(&ClientReliableMessages::PilotQuery {
                        ship_entity: server_entity,
                    }),
                );
            }
            ServerReliableMessages::ChunkData {
                structure_entity: server_structure_entity,
                serialized_chunk,
            } => {
                if let Some(s_entity) = network_mapping.client_from_server(&server_structure_entity)
                {
                    if let Ok(mut structure) = query_structure.get_mut(s_entity) {
                        let chunk: Chunk = cosmos_encoder::deserialize(&serialized_chunk)
                            .expect("Unable to deserialize chunk from server");

                        let (x, y, z) = (
                            chunk.structure_x(),
                            chunk.structure_y(),
                            chunk.structure_z(),
                        );

                        structure.set_chunk(chunk);

                        set_chunk_event_writer.send(ChunkInitEvent {
                            x,
                            y,
                            z,
                            structure_entity: s_entity,
                        });
                    }
                }
            }
            ServerReliableMessages::StructureRemove {
                entity: server_entity,
            } => {
                if let Some(entity) = network_mapping.client_from_server(&server_entity) {
                    commands.entity(entity).despawn_recursive();
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
                block_up,
            } => {
                // Sometimes you'll get block updates for structures that don't exist
                if let Some(client_ent) = network_mapping.client_from_server(&structure_entity) {
                    if let Ok(mut structure) = query_structure.get_mut(client_ent) {
                        structure.set_block_at(
                            x as usize,
                            y as usize,
                            z as usize,
                            blocks.from_numeric_id(block_id),
                            block_up,
                            &blocks,
                            Some(&mut block_change_event_writer),
                        );
                    }
                }
            }
            ServerReliableMessages::PilotChange {
                structure_entity,
                pilot_entity,
            } => {
                let pilot_entity = if let Some(pilot_entity) = pilot_entity {
                    network_mapping.client_from_server(&pilot_entity)
                } else {
                    None
                };

                let Some(structure_entity) = network_mapping.client_from_server(&structure_entity) else {
                    continue;
                };

                pilot_change_event_writer.send(ChangePilotEvent {
                    structure_entity,
                    pilot_entity,
                });
            }
            ServerReliableMessages::EntityInventory {
                serialized_inventory,
                owner,
            } => {
                if let Some(client_entity) = network_mapping.client_from_server(&owner) {
                    let inventory: Inventory =
                        cosmos_encoder::deserialize(&serialized_inventory).unwrap();

                    commands.entity(client_entity).insert(inventory);
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
            ServerReliableMessages::Star { entity, star } => {
                if let Some(client_entity) = network_mapping.client_from_server(&entity) {
                    commands.entity(client_entity).insert((
                        star,
                        LoadingDistance::new(SYSTEM_SECTORS / 2, SYSTEM_SECTORS / 2),
                    ));
                } else {
                    network_mapping.add_mapping(
                        commands
                            .spawn((
                                star,
                                LoadingDistance::new(SYSTEM_SECTORS / 2, SYSTEM_SECTORS / 2),
                            ))
                            .id(),
                        entity,
                    );
                }
            }
        }
    }
}

/// Handles any just-added locations that need to sync up to their transforms
fn fix_location(
    mut query: Query<(&mut Location, &mut Transform), (Added<Location>, Without<PlayerWorld>)>,
    player_worlds: Query<&Location, With<PlayerWorld>>,
) {
    for (mut location, mut transform) in query.iter_mut() {
        match player_worlds.get_single() {
            Ok(loc) => {
                transform.translation = location.relative_coords_to(loc);
                location.last_transform_loc = Some(transform.translation);
            }
            _ => {
                warn!("Something was added with a location before a player world was registered.")
            }
        }
    }
}

fn sync_transforms_and_locations(
    mut trans_query_no_parent: Query<
        (&mut Transform, &mut Location),
        (Without<PlayerWorld>, Without<Parent>),
    >,
    mut trans_query_with_parent: Query<
        (&mut Transform, &mut Location),
        (Without<PlayerWorld>, With<Parent>),
    >,
    parent_query: Query<&Parent>,
    player_entity_query: Query<Entity, With<LocalPlayer>>,
    mut world_query: Query<(&PlayerWorld, &mut Location)>,
) {
    for (transform, mut location) in trans_query_no_parent.iter_mut() {
        location.apply_updates(transform.translation);
    }
    for (transform, mut location) in trans_query_with_parent.iter_mut() {
        location.apply_updates(transform.translation);
    }

    if let Ok((world, mut world_location)) = world_query.get_single_mut() {
        let mut player_entity = player_entity_query
            .get(world.player)
            .expect("This player should exist.");

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
            .map(|x| x.1)
            .or_else(|_| match trans_query_with_parent.get(player_entity) {
                Ok((_, loc)) => Ok(loc),
                Err(x) => Err(x),
            })
            .expect("The above loop guarantees this is valid");

        world_location.set_from(location);

        // Update transforms of objects within this world.
        for (mut transform, mut location) in trans_query_no_parent.iter_mut() {
            let trans = world_location.relative_coords_to(&location);
            transform.translation = trans;
            location.last_transform_loc = Some(trans);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(RequestedEntities::default())
        .add_systems((update_crosshair, insert_last_rotation))
        .add_system(
            client_sync_players
                .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld))),
        )
        .add_systems(
            (
                fix_location.after(client_sync_players),
                sync_transforms_and_locations,
                bubble_down_locations,
            )
                .chain()
                .in_set(OnUpdate(GameState::Playing)),
        );
}
