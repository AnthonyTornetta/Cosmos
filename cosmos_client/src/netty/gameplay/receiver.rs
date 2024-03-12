//! A receiver + processor for a bunch of network packets.
//!
//! This should eventually be broken up

use std::sync::{Arc, Mutex};

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*, window::PrimaryWindow};
use bevy_kira_audio::prelude::AudioReceiver;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use cosmos_core::{
    block::Block,
    ecs::{bundles::CosmosPbrBundle, NeedsDespawned},
    entities::player::{render_distance::RenderDistance, Player},
    events::{block_events::BlockChangedEvent, structure::change_pilot_event::ChangePilotEvent},
    inventory::Inventory,
    netty::{
        client_reliable_messages::ClientReliableMessages,
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages,
        NettyChannelClient, NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::{
        location::{add_previous_location, handle_child_syncing, Location, LocationPhysicsSet, SYSTEM_SECTORS},
        player_world::PlayerWorld,
    },
    registry::Registry,
    structure::{
        block_health::events::BlockTakeDamageEvent,
        chunk::Chunk,
        dynamic_structure::DynamicStructure,
        full_structure::FullStructure,
        planet::{biosphere::BiosphereMarker, planet_builder::TPlanetBuilder},
        shared::build_mode::{EnterBuildModeEvent, ExitBuildModeEvent},
        ship::{pilot::Pilot, ship_builder::TShipBuilder, Ship},
        station::station_builder::TStationBuilder,
        ChunkInitEvent, Structure,
    },
};

use crate::{
    camera::camera_controller::CameraHelper,
    netty::{
        flags::LocalPlayer,
        lobby::{ClientLobby, PlayerInfo},
        mapping::{Mappable, NetworkMapping},
    },
    rendering::{CameraPlayerOffset, MainCamera},
    state::game_state::GameState,
    structure::{
        planet::{client_planet_builder::ClientPlanetBuilder, generation::SetTerrainGenData},
        ship::client_ship_builder::ClientShipBuilder,
        station::client_station_builder::ClientStationBuilder,
    },
    ui::{
        crosshair::CrosshairOffset,
        message::{HudMessage, HudMessages},
        UiRoot,
    },
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
    local_player_query: Query<(), With<LocalPlayer>>,
    camera_query: Query<(Entity, &Camera), With<MainCamera>>,
    transform_query: Query<&GlobalTransform>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    for (pilot, mut last_rotation, transform) in query.iter_mut() {
        if local_player_query.contains(pilot.entity) {
            let (cam_entity, camera) = camera_query.get_single().unwrap();

            let cam_global = transform_query.get(cam_entity).unwrap();

            let primary = primary_query.get_single().expect("Missing primary window");

            if let Some(mut pos_on_screen) = camera.world_to_viewport(
                cam_global,
                last_rotation.0.mul_vec3(Vec3::new(0.0, 0.0, -1.0)) + cam_global.translation(),
            ) {
                pos_on_screen -= Vec2::new(primary.width() / 2.0, primary.height() / 2.0);

                crosshair_offset.x += pos_on_screen.x;
                crosshair_offset.y -= pos_on_screen.y;
            }

            last_rotation.0 = transform.rotation;
        }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
struct RequestedEntity {
    server_entity: Entity,
    client_entity: Entity,
    seconds_since_request: f32,
}

#[derive(Resource, Debug, Default)]
pub(crate) struct RequestedEntities {
    entities: Vec<RequestedEntity>,
}

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
/// Unused
pub struct NetworkTick(pub u64);

#[derive(Debug, Component, Deref)]
pub(crate) struct LerpTowards(NettyRigidBody);

fn lerp_towards(
    mut location_query: Query<&mut Location>,
    global_transform_query: Query<&GlobalTransform>,
    mut query: Query<(Entity, &LerpTowards, &mut Transform, &mut Velocity), With<Location>>,
) {
    for (entity, lerp_towards, mut transform, mut velocity) in query.iter_mut() {
        match lerp_towards.location {
            NettyRigidBodyLocation::Absolute(location) => {
                let to_location = location;
                let mut location = location_query.get_mut(entity).expect("The above With statement guarentees this");

                if to_location.distance_sqrd(&location) > 100.0 {
                    location.set_from(&to_location);
                } else {
                    let lerpped_loc = *location + (location.relative_coords_to(&to_location)) * 0.1;

                    location.set_from(&lerpped_loc);
                }
            }
            NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                if let Ok(g_trans) = global_transform_query.get(entity) {
                    let parent_rot = Quat::from_affine3(&g_trans.affine());
                    let final_trans = parent_rot.inverse().mul_vec3(rel_trans);

                    transform.translation = final_trans;
                }
            }
        };

        transform.rotation = //lerp_towards.rotation;
            transform.rotation.lerp(lerp_towards.rotation, 0.1);

        velocity.linvel = lerp_towards.body_vel.linvel.into();
        // velocity
        //     .linvel
        //     .lerp(lerp_towards.body_vel.linvel.into(), 0.1);
        velocity.angvel = lerp_towards.body_vel.angvel.into();
        // velocity
        //     .angvel
        //     .lerp(lerp_towards.body_vel.angvel.into(), 0.1);
    }
}

/// TODO: super split this up
pub(crate) fn client_sync_players(
    mut commands: Commands,
    (mut meshes, mut client, transport, mut lobby, mut network_mapping): (
        ResMut<Assets<Mesh>>,
        ResMut<RenetClient>,
        Res<NetcodeClientTransport>,
        ResMut<ClientLobby>,
        ResMut<NetworkMapping>,
    ),
    (mut set_chunk_event_writer, mut block_change_event_writer, mut take_damage_event_writer, mut set_terrain_data_ev_writer): (
        EventWriter<ChunkInitEvent>,
        EventWriter<BlockChangedEvent>,
        EventWriter<BlockTakeDamageEvent>,
        EventWriter<SetTerrainGenData>,
    ),
    (query_player, parent_query): (Query<&Player>, Query<&Parent>),
    mut query_body: Query<
        (
            Option<&mut Location>,
            Option<&mut Transform>,
            Option<&Velocity>,
            Option<&mut NetworkTick>,
            Option<&mut LerpTowards>,
        ),
        Without<LocalPlayer>,
    >,
    mut query_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
    mut requested_entities: ResMut<RequestedEntities>,
    time: Res<Time>,
    local_player: Query<Entity, With<LocalPlayer>>,

    mut hud_messages: ResMut<HudMessages>,

    (mut build_mode_enter, mut build_mode_exit): (EventWriter<EnterBuildModeEvent>, EventWriter<ExitBuildModeEvent>),
) {
    let client_id = transport.client_id();

    requested_entities.entities.retain_mut(|x| {
        x.seconds_since_request += time.delta_seconds();
        if x.seconds_since_request < 10.0 {
            true
        } else {
            commands.entity(x.client_entity).despawn_recursive();
            false
        }
    });

    while let Some(message) = client.receive_message(NettyChannelServer::Unreliable) {
        let msg: ServerUnreliableMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::BulkBodies { bodies, time_stamp } => {
                for (server_entity, body) in bodies.iter() {
                    let Ok(body) = body.map(&network_mapping) else {
                        continue;
                    };

                    if let Some(entity) = network_mapping.client_from_server(server_entity) {
                        if let Ok((location, transform, velocity, net_tick, lerp_towards)) = query_body.get_mut(entity) {
                            if let Some(mut net_tick) = net_tick {
                                if net_tick.0 >= time_stamp {
                                    // Received position packet for previous time, disregard.
                                    continue;
                                } else {
                                    net_tick.0 = time_stamp;
                                }
                            } else {
                                commands.entity(entity).insert(NetworkTick(time_stamp));
                            }

                            if location.is_some() && transform.is_some() && velocity.is_some() {
                                if let Some(mut lerp_towards) = lerp_towards {
                                    lerp_towards.0 = body;
                                } else {
                                    commands.entity(entity).insert(LerpTowards(body));
                                }
                            } else {
                                let loc = match body.location {
                                    NettyRigidBodyLocation::Absolute(location) => location,
                                    NettyRigidBodyLocation::Relative(rel_trans, parent_ent) => {
                                        let parent_loc =
                                            query_body.get(parent_ent).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                                        parent_loc + rel_trans
                                    }
                                };

                                commands.entity(entity).insert((loc, body.create_velocity(), LerpTowards(body)));
                            }
                        }
                    } else if !requested_entities.entities.iter().any(|x| x.server_entity == *server_entity) {
                        let client_entity = commands.spawn_empty().id();

                        requested_entities.entities.push(RequestedEntity {
                            server_entity: *server_entity,
                            client_entity,
                            seconds_since_request: 0.0,
                        });
                        network_mapping.add_mapping(client_entity, *server_entity);

                        client.send_message(
                            NettyChannelClient::Reliable,
                            cosmos_encoder::serialize(&ClientReliableMessages::RequestEntityData { entity: *server_entity }),
                        );
                    }
                }
            }
            ServerUnreliableMessages::SetMovement { movement, ship_entity } => {
                if let Some(entity) = network_mapping.client_from_server(&ship_entity) {
                    commands.entity(entity).insert(movement);
                }
            }
        }
    }

    while let Some(message) = client.receive_message(NettyChannelServer::Reliable) {
        let msg: ServerReliableMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerReliableMessages::PlayerCreate {
                body,
                id,
                entity: server_entity,
                name,
                inventory_serialized,
                render_distance: _,
            } => {
                // Prevents creation of duplicate players
                if lobby.players.contains_key(&id) {
                    warn!("DUPLICATE PLAYER RECEIVED {id}");
                    continue;
                }

                let Ok(body) = body.map(&network_mapping) else {
                    continue;
                };

                info!("Player {} ({}) connected!", name.as_str(), id);

                let mut entity_cmds = commands.spawn_empty();

                let inventory: Inventory = cosmos_encoder::deserialize(&inventory_serialized).unwrap();

                let mut loc = match body.location {
                    NettyRigidBodyLocation::Absolute(location) => location,
                    NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                        let parent_loc = query_body.get(entity).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                        parent_loc + rel_trans
                    }
                };

                // This should be set via the server, but just in case,
                // this will avoid any position mismatching
                // ** future note: this may not be needed??
                loc.last_transform_loc = Some(loc.local);

                entity_cmds.insert((
                    CosmosPbrBundle {
                        location: loc,
                        rotation: body.rotation.into(),
                        mesh: meshes.add(Capsule3d::default()),
                        ..default()
                    },
                    Collider::capsule_y(0.65, 0.25),
                    LockedAxes::ROTATION_LOCKED,
                    Name::new(format!("Player ({name})")),
                    RigidBody::Dynamic,
                    body.create_velocity(),
                    Player::new(name, id),
                    ReadMassProperties::default(),
                    ActiveEvents::COLLISION_EVENTS,
                    inventory,
                ));

                let client_entity = entity_cmds.id();

                let player_info = PlayerInfo {
                    server_entity,
                    client_entity,
                };

                lobby.players.insert(id, player_info);
                network_mapping.add_mapping(client_entity, server_entity);

                let camera_offset = Vec3::new(0.0, 0.75, 0.0);

                if client_id == id {
                    entity_cmds
                        .insert((LocalPlayer, RenderDistance::default(), CameraPlayerOffset(camera_offset)))
                        .with_children(|parent| {
                            parent.spawn((
                                Camera3dBundle {
                                    camera: Camera {
                                        hdr: true,
                                        ..Default::default()
                                    },
                                    transform: Transform::from_translation(camera_offset),
                                    projection: Projection::from(PerspectiveProjection {
                                        fov: (90.0 / 180.0) * std::f32::consts::PI,
                                        ..default()
                                    }),
                                    ..default()
                                },
                                BloomSettings { ..Default::default() },
                                CameraHelper::default(),
                                Name::new("Main Camera"),
                                MainCamera,
                                UiRoot,
                                // No double UI rendering
                                AudioReceiver,
                            ));
                        });

                    commands.spawn((
                        PlayerWorld { player: client_entity },
                        Name::new("Player World"),
                        loc,
                        PhysicsWorld {
                            world_id: DEFAULT_WORLD_ID,
                        },
                    ));
                }
            }
            ServerReliableMessages::PlayerRemove { id } => {
                if let Some(PlayerInfo {
                    client_entity,
                    server_entity: _,
                }) = lobby.players.remove(&id)
                {
                    if let Some(mut entity) = commands.get_entity(client_entity) {
                        if let Ok(player) = query_player.get(client_entity) {
                            info!("Player {} ({id}) disconnected", player.name());
                        }

                        entity.insert(NeedsDespawned);
                    }
                }
            }
            // This could cause issues in the future if a client receives a planet's position first then this packet.
            // Please restructure this + the ship to use the new requesting system.
            ServerReliableMessages::Planet {
                entity: server_entity,
                dimensions,
                planet,
                biosphere,
                location,
            } => {
                let Some(entity) = network_mapping.client_from_server(&server_entity) else {
                    continue;
                };

                let mut entity_cmds = commands.entity(entity);
                let mut structure = Structure::Dynamic(DynamicStructure::new(dimensions));

                let builder = ClientPlanetBuilder::default();
                builder.insert_planet(&mut entity_cmds, location, &mut structure, planet);

                entity_cmds.insert((structure, BiosphereMarker::new(biosphere)));
            }
            ServerReliableMessages::NumberOfChunks {
                entity: server_entity,
                chunks_needed,
            } => {
                let Some(entity) = network_mapping.client_from_server(&server_entity) else {
                    continue;
                };

                if let Some(mut ecmds) = commands.get_entity(entity) {
                    ecmds.insert(chunks_needed);
                }
            }
            ServerReliableMessages::Ship {
                entity: server_entity,
                body,
                dimensions,
            } => {
                let Some(entity) = network_mapping.client_from_server(&server_entity) else {
                    continue;
                };

                let Ok(body) = body.map(&network_mapping) else {
                    continue;
                };

                let location = match body.location {
                    NettyRigidBodyLocation::Absolute(location) => location,
                    NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                        let parent_loc = query_body.get(entity).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                        parent_loc + rel_trans
                    }
                };

                let mut entity_cmds = commands.entity(entity);
                let mut structure = Structure::Full(FullStructure::new(dimensions));

                let builder = ClientShipBuilder::default();
                builder.insert_ship(&mut entity_cmds, location, body.create_velocity(), &mut structure);

                entity_cmds.insert((structure /*chunks_needed*/,));

                client.send_message(
                    NettyChannelClient::Reliable,
                    cosmos_encoder::serialize(&ClientReliableMessages::PilotQuery {
                        ship_entity: server_entity,
                    }),
                );
            }
            ServerReliableMessages::Station {
                entity: server_entity,
                body,
                dimensions,
            } => {
                let Some(entity) = network_mapping.client_from_server(&server_entity) else {
                    continue;
                };

                let Ok(body) = body.map(&network_mapping) else {
                    continue;
                };

                let location = match body.location {
                    NettyRigidBodyLocation::Absolute(location) => location,
                    NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                        let parent_loc = query_body.get(entity).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                        parent_loc + rel_trans
                    }
                };

                let mut entity_cmds = commands.entity(entity);
                let mut structure = Structure::Full(FullStructure::new(dimensions));

                let builder = ClientStationBuilder::default();
                builder.insert_station(&mut entity_cmds, location, &mut structure);

                entity_cmds.insert((structure /*chunks_needed*/,));
            }
            ServerReliableMessages::ChunkData {
                structure_entity: server_structure_entity,
                serialized_chunk,
                serialized_block_data,
            } => {
                if let Some(s_entity) = network_mapping.client_from_server(&server_structure_entity) {
                    if let Ok(mut structure) = query_structure.get_mut(s_entity) {
                        let chunk: Chunk = cosmos_encoder::deserialize(&serialized_chunk).expect("Unable to deserialize chunk from server");
                        let chunk_coords = chunk.chunk_coordinates();

                        structure.set_chunk(chunk);

                        set_chunk_event_writer.send(ChunkInitEvent {
                            coords: chunk_coords,
                            structure_entity: s_entity,
                            serialized_block_data: serialized_block_data.map(|x| Arc::new(Mutex::new(x))),
                        });
                    }
                }
            }
            ServerReliableMessages::EmptyChunk { structure_entity, coords } => {
                if let Some(s_entity) = network_mapping.client_from_server(&structure_entity) {
                    if let Ok(mut structure) = query_structure.get_mut(s_entity) {
                        structure.set_to_empty_chunk(coords);

                        set_chunk_event_writer.send(ChunkInitEvent {
                            coords,
                            structure_entity: s_entity,
                            serialized_block_data: None,
                        });
                    }
                }
            }
            ServerReliableMessages::StructureRemove { entity: server_entity } => {
                if let Some(entity) = network_mapping.client_from_server(&server_entity) {
                    commands.entity(entity).insert(NeedsDespawned);
                }
            }
            ServerReliableMessages::MOTD { motd } => {
                hud_messages.display_message(motd.into());
            }
            ServerReliableMessages::BlockChange {
                blocks_changed_packet,
                structure_entity,
            } => {
                // Sometimes you'll get block updates for structures that don't exist
                if let Some(client_ent) = network_mapping.client_from_server(&structure_entity) {
                    if let Ok(mut structure) = query_structure.get_mut(client_ent) {
                        for block_changed in blocks_changed_packet.0 {
                            structure.set_block_at(
                                block_changed.coordinates.coords(),
                                blocks.from_numeric_id(block_changed.block_id),
                                block_changed.block_up,
                                &blocks,
                                Some(&mut block_change_event_writer),
                            );
                        }
                    }
                }
            }
            ServerReliableMessages::PilotChange {
                structure_entity,
                pilot_entity,
            } => {
                let pilot_entity = if let Some(pilot_entity) = pilot_entity {
                    if let Some(mapping) = network_mapping.client_from_server(&pilot_entity) {
                        Some(mapping)
                    } else {
                        warn!("Server mapping missing for pilot!");
                        None
                    }
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
            ServerReliableMessages::Star { entity, star } => {
                if let Some(client_entity) = network_mapping.client_from_server(&entity) {
                    commands
                        .entity(client_entity)
                        .insert((star, LoadingDistance::new(SYSTEM_SECTORS / 2, SYSTEM_SECTORS / 2)));
                } else {
                    network_mapping.add_mapping(
                        commands
                            .spawn((star, LoadingDistance::new(SYSTEM_SECTORS / 2, SYSTEM_SECTORS / 2)))
                            .id(),
                        entity,
                    );
                }
            }
            ServerReliableMessages::PlayerLeaveShip { player_entity } => {
                if let Some(player_entity) = network_mapping.client_from_server(&player_entity) {
                    if let Some(mut ecmds) = commands.get_entity(player_entity) {
                        let Ok(parent) = parent_query.get(player_entity) else {
                            continue;
                        };

                        ecmds.remove_parent();

                        let Ok(Some(ship_trans)) = query_body.get(parent.get()).map(|x| x.1.cloned()) else {
                            continue;
                        };

                        let ship_translation = ship_trans.translation;

                        if let Ok((Some(mut loc), Some(mut trans))) = query_body.get_mut(player_entity).map(|x| (x.0, x.1)) {
                            let cur_trans = trans.translation;

                            trans.translation = cur_trans + ship_translation;

                            loc.last_transform_loc = Some(trans.translation);
                        }
                    }
                }
            }
            ServerReliableMessages::PlayerJoinShip {
                player_entity,
                ship_entity,
            } => {
                let Some(player_entity) = network_mapping.client_from_server(&player_entity) else {
                    continue;
                };

                let Some(mut ecmds) = commands.get_entity(player_entity) else {
                    continue;
                };

                let Some(ship_entity) = network_mapping.client_from_server(&ship_entity) else {
                    continue;
                };

                ecmds.set_parent(ship_entity);

                let Ok(Some(ship_loc)) = query_body.get(ship_entity).map(|x| x.0.cloned()) else {
                    continue;
                };

                if let Ok((Some(mut loc), Some(mut trans), _, _, _)) = query_body.get_mut(player_entity) {
                    trans.translation = (*loc - ship_loc).absolute_coords_f32();
                    loc.last_transform_loc = Some(trans.translation);
                }
            }
            ServerReliableMessages::PlayerEnterBuildMode {
                player_entity,
                structure_entity,
            } => {
                if let Some(player_entity) = network_mapping.client_from_server(&player_entity) {
                    if let Some(structure_entity) = network_mapping.client_from_server(&structure_entity) {
                        build_mode_enter.send(EnterBuildModeEvent {
                            player_entity,
                            structure_entity,
                        });
                    }
                }
            }
            ServerReliableMessages::PlayerExitBuildMode { player_entity } => {
                if let Some(player_entity) = network_mapping.client_from_server(&player_entity) {
                    build_mode_exit.send(ExitBuildModeEvent { player_entity });
                }
            }
            ServerReliableMessages::UpdateBuildMode { build_mode } => {
                if let Ok(player_entity) = local_player.get_single() {
                    commands.entity(player_entity).insert(build_mode);
                }
            }
            ServerReliableMessages::InvalidReactor { reason } => {
                hud_messages.display_message(HudMessage::with_colored_string(
                    format!("Invalid reactor setup: {reason}"),
                    Color::ORANGE_RED,
                ));
            }
            ServerReliableMessages::Reactors { reactors, structure } => {
                if let Some(structure_entity) = network_mapping.client_from_server(&structure) {
                    commands.entity(structure_entity).insert(reactors);
                }
            }
            ServerReliableMessages::RequestedEntityReceived(entity) => {
                requested_entities.entities.retain(|x| x.server_entity != entity);
            }
            ServerReliableMessages::BlockHealthChange { changes } => {
                take_damage_event_writer.send_batch(changes.into_iter().filter_map(|ev| {
                    network_mapping
                        .client_from_server(&ev.structure_entity)
                        .map(|structure_entity| BlockTakeDamageEvent {
                            structure_entity,
                            block: ev.block,
                            new_health: ev.new_health,
                        })
                }));
            }
            ServerReliableMessages::Credits { credits, entity } => {
                if let Some(entity) = network_mapping.client_from_server(&entity) {
                    commands.entity(entity).insert(credits);
                }
            }
            ServerReliableMessages::TerrainGenJazz {
                shaders,
                permutation_table,
            } => {
                set_terrain_data_ev_writer.send(SetTerrainGenData {
                    files: shaders,
                    permutation_table,
                });
            }
        }
    }
}

/// Handles any just-added locations that need to sync up to their transforms
fn fix_location(
    mut query: Query<(Entity, &mut Location, Option<&mut Transform>), (Added<Location>, Without<PlayerWorld>, Without<Parent>)>,
    player_worlds: Query<&Location, With<PlayerWorld>>,
    mut commands: Commands,
) {
    for (entity, mut location, transform) in query.iter_mut() {
        match player_worlds.get_single() {
            Ok(loc) => {
                let translation = loc.relative_coords_to(&location);
                if let Some(mut transform) = transform {
                    transform.translation = translation;
                } else {
                    commands
                        .entity(entity)
                        .insert(TransformBundle::from_transform(Transform::from_translation(translation)));
                }
                location.last_transform_loc = Some(translation);
            }
            _ => {
                warn!("Something was added with a location before a player world was registered.")
            }
        }
    }
}

fn sync_transforms_and_locations(
    mut trans_query_no_parent: Query<(&mut Transform, &mut Location), (Without<PlayerWorld>, Without<Parent>)>,
    trans_query_with_parent: Query<&Location, (Without<PlayerWorld>, With<Parent>)>,
    parent_query: Query<&Parent>,
    player_entity_query: Query<Entity, With<LocalPlayer>>,
    mut world_query: Query<(&PlayerWorld, &mut Location)>,
) {
    for (transform, mut location) in trans_query_no_parent.iter_mut() {
        if location.last_transform_loc.is_some() {
            location.apply_updates(transform.translation);
        }
    }

    if let Ok((world, mut world_location)) = world_query.get_single_mut() {
        let mut player_entity = player_entity_query.get(world.player).expect("This player should exist.");

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
                Ok(loc) => Ok(loc),
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

/// Fixes oddities that happen when changing parent of player
fn player_changed_parent(
    q_parent: Query<(&GlobalTransform, &Location)>,
    mut q_local_player: Query<(&mut Transform, &Location, &Parent), (Changed<Parent>, With<LocalPlayer>)>,
) {
    let Ok((mut player_trans, player_loc, parent)) = q_local_player.get_single_mut() else {
        return;
    };

    let Ok((parent_trans, parent_loc)) = q_parent.get(parent.get()) else {
        return;
    };

    // Because the player's translation is always 0, 0, 0 we need to adjust it so the player is put into the
    // right spot in its parent.
    player_trans.translation = Quat::from_affine3(&parent_trans.affine())
        .inverse()
        .mul_vec3((*player_loc - *parent_loc).absolute_coords_f32());
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(RequestedEntities::default())
        .configure_sets(Update, LocationPhysicsSet::DoPhysics)
        .add_systems(Update, (update_crosshair, insert_last_rotation))
        .add_systems(
            Update,
            client_sync_players.run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld))),
        )
        .add_systems(
            Update,
            (
                fix_location.before(client_sync_players),
                lerp_towards.after(client_sync_players),
                (
                    player_changed_parent,
                    sync_transforms_and_locations,
                    handle_child_syncing,
                    add_previous_location,
                )
                    .chain()
                    .in_set(LocationPhysicsSet::DoPhysics),
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
}
