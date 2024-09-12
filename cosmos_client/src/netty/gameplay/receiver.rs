//! A receiver + processor for a bunch of network packets.
//!
//! This should eventually be broken up

use std::sync::{Arc, Mutex};

use bevy::{color::palettes::css, core_pipeline::bloom::BloomSettings, prelude::*, window::PrimaryWindow};
use bevy_kira_audio::prelude::AudioReceiver;
use bevy_rapier3d::prelude::*;
use bevy_renet2::renet2::{transport::NetcodeClientTransport, RenetClient};
use cosmos_core::{
    block::Block,
    ecs::{
        bundles::{BundleStartingRotation, CosmosPbrBundle},
        NeedsDespawned,
    },
    entities::player::{render_distance::RenderDistance, Player},
    events::{
        block_events::{BlockChangedEvent, BlockDataChangedEvent},
        structure::change_pilot_event::ChangePilotEvent,
    },
    inventory::{held_item_slot::HeldItemSlot, Inventory},
    netty::{
        client::{LocalPlayer, NeedsLoadedFromServer},
        client_reliable_messages::ClientReliableMessages,
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages,
        sync::{
            client_syncing::ClientReceiveComponents,
            mapping::{Mappable, NetworkMapping, ServerEntity},
            ComponentEntityIdentifier,
        },
        system_sets::NetworkingSystemsSet,
        NettyChannelClient, NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::{
        location::{add_previous_location, handle_child_syncing, CosmosBundleSet, Location, LocationPhysicsSet, SYSTEM_SECTORS},
        player_world::{PlayerWorld, WorldWithin},
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
        systems::{dock_system::Docked, StructureSystems},
        ChunkInitEvent, Structure,
    },
};

use crate::{
    camera::camera_controller::CameraHelper,
    netty::lobby::{ClientLobby, PlayerInfo},
    rendering::{CameraPlayerOffset, MainCamera},
    settings::DesiredFov,
    state::game_state::GameState,
    structure::{
        planet::{client_planet_builder::ClientPlanetBuilder, generation::SetTerrainGenData},
        ship::{client_ship_builder::ClientShipBuilder, ship_movement::ClientCreateShipMovementSet},
        station::client_station_builder::ClientStationBuilder,
    },
    ui::{
        crosshair::{CrosshairOffset, CrosshairOffsetSet},
        message::{HudMessage, HudMessages},
        UiRoot,
    },
    window::setup::CursorFlagsSet,
};

#[derive(Component)]
struct LastRotation(Quat);

fn insert_last_rotation(mut commands: Commands, query: Query<Entity, Added<Structure>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(LastRotation(Quat::IDENTITY));
    }
}

fn update_crosshair(
    mut q_ships: Query<(&Pilot, &mut LastRotation, &Transform, Option<&Docked>), (With<Ship>, Changed<Transform>)>,
    local_player_query: Query<(), With<LocalPlayer>>,
    camera_query: Query<(&GlobalTransform, &Transform, &Camera), With<MainCamera>>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
) {
    for (pilot, mut last_rotation, transform, docked) in q_ships.iter_mut() {
        if !local_player_query.contains(pilot.entity) {
            continue;
        }

        let Ok((cam_global_trans, cam_trans, camera)) = camera_query.get_single() else {
            return;
        };

        let Ok(primary) = primary_query.get_single() else {
            return;
        };

        if docked.is_some() {
            crosshair_offset.x = 0.0;
            crosshair_offset.y = 0.0;
        } else if let Some(mut pos_on_screen) = camera.world_to_viewport(
            cam_global_trans,
            last_rotation.0.mul_vec3(Vec3::from(cam_trans.forward())) + cam_global_trans.translation(),
        ) {
            pos_on_screen -= Vec2::new(primary.width() / 2.0, primary.height() / 2.0);

            crosshair_offset.x += pos_on_screen.x;
            crosshair_offset.y -= pos_on_screen.y;
        }

        last_rotation.0 = transform.rotation;
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

        let vel = lerp_towards.body_vel.unwrap_or_default();

        velocity.linvel = vel.linvel;
        // velocity
        //     .linvel
        //     .lerp(lerp_towards.body_vel.linvel.into(), 0.1);
        velocity.angvel = vel.angvel;
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
    (
        mut set_chunk_event_writer,
        mut block_change_event_writer,
        mut take_damage_event_writer,
        mut set_terrain_data_ev_writer,
        mut evw_block_data_changed,
    ): (
        EventWriter<ChunkInitEvent>,
        EventWriter<BlockChangedEvent>,
        EventWriter<BlockTakeDamageEvent>,
        EventWriter<SetTerrainGenData>,
        EventWriter<BlockDataChangedEvent>,
    ),
    (q_default_rapier_context, query_player, parent_query, q_structure_systems, mut q_inventory, mut q_structure): (
        Query<Entity, With<DefaultRapierContext>>,
        Query<&Player>,
        Query<&Parent>,
        Query<&StructureSystems>,
        Query<&mut Inventory>,
        Query<&mut Structure>,
    ),
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
    desired_fov: Res<DesiredFov>,
    q_needs_loaded: Query<(), With<NeedsLoadedFromServer>>,
    q_parent: Query<&Parent>,
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
            if let Some(ecmds) = commands.get_entity(x.client_entity) {
                ecmds.despawn_recursive();
            }
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
                        if q_needs_loaded.contains(entity) {
                            commands.entity(entity).remove::<NeedsLoadedFromServer>();

                            requested_entities.entities.push(RequestedEntity {
                                server_entity: *server_entity,
                                client_entity: entity,
                                seconds_since_request: 0.0,
                            });

                            client.send_message(
                                NettyChannelClient::Reliable,
                                cosmos_encoder::serialize(&ClientReliableMessages::RequestEntityData { entity: *server_entity }),
                            );
                        } else if let Ok((location, transform, velocity, net_tick, lerp_towards)) = query_body.get_mut(entity) {
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

                                        if let Ok(parent) = q_parent.get(entity) {
                                            if parent.get() != parent_ent {
                                                commands.entity(entity).set_parent(parent_ent);
                                            }
                                        } else {
                                            commands.entity(entity).set_parent(parent_ent);
                                        }

                                        parent_loc + rel_trans
                                    }
                                };

                                commands.entity(entity).insert((
                                    loc,
                                    BundleStartingRotation(body.rotation),
                                    body.create_velocity(),
                                    LerpTowards(body),
                                ));
                            }
                        }
                    } else if !requested_entities.entities.iter().any(|x| x.server_entity == *server_entity) {
                        let (loc, parent_ent) = match body.location {
                            NettyRigidBodyLocation::Absolute(location) => (location, None),
                            NettyRigidBodyLocation::Relative(rel_trans, parent_ent) => {
                                let parent_loc = query_body.get(parent_ent).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                                (parent_loc + rel_trans, Some(parent_ent))
                            }
                        };

                        let mut client_entity_ecmds = commands.spawn((
                            ServerEntity(*server_entity),
                            loc,
                            BundleStartingRotation(body.rotation),
                            body.create_velocity(),
                            LerpTowards(body),
                        ));

                        if let Some(parent_ent) = parent_ent {
                            client_entity_ecmds.set_parent(parent_ent);
                        }

                        let client_entity = client_entity_ecmds.id();

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
            // TODO: Get player data via the normal request entity function!
            ServerReliableMessages::PlayerCreate {
                body,
                id,
                entity: server_entity,
                name,
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

                // The player entity may have already been created if some of their components were already synced.
                let mut entity_cmds = if let Some(player_entity) = network_mapping.client_from_server(&server_entity) {
                    commands.entity(player_entity)
                } else {
                    commands.spawn_empty()
                };

                let mut loc = match body.location {
                    NettyRigidBodyLocation::Absolute(location) => location,
                    NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                        let parent_loc = query_body.get(entity).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                        parent_loc + rel_trans
                    }
                };

                // this will avoid any position mismatching
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
                    Friction {
                        coefficient: 0.0,
                        combine_rule: CoefficientCombineRule::Min,
                    },
                    body.create_velocity(),
                    Player::new(name, id),
                    ReadMassProperties::default(),
                    ActiveEvents::COLLISION_EVENTS,
                    ServerEntity(server_entity),
                ));

                let client_entity = entity_cmds.id();

                let player_info = PlayerInfo {
                    server_entity,
                    client_entity,
                };

                lobby.players.insert(id, player_info);
                network_mapping.add_mapping(client_entity, server_entity);

                let camera_offset = Vec3::new(0.0, 0.75, 0.0);

                // Requests all components needed for the player
                client.send_message(
                    NettyChannelClient::Reliable,
                    cosmos_encoder::serialize(&ClientReliableMessages::RequestEntityData { entity: server_entity }),
                );

                if client_id == id {
                    entity_cmds
                        .insert((
                            LocalPlayer,
                            HeldItemSlot::new(0).unwrap(),
                            RenderDistance::default(),
                            CameraPlayerOffset(camera_offset),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Camera3dBundle {
                                    camera: Camera {
                                        hdr: true,
                                        ..Default::default()
                                    },
                                    transform: Transform::from_translation(camera_offset),
                                    projection: Projection::from(PerspectiveProjection {
                                        fov: (desired_fov.0 / 180.0) * std::f32::consts::PI,
                                        ..default()
                                    }),
                                    ..default()
                                },
                                BloomSettings { ..Default::default() },
                                CameraHelper::default(),
                                Name::new("Main Camera"),
                                MainCamera,
                                UiRoot,
                                AudioReceiver,
                            ));
                        });

                    info!("Player world!");
                    commands.spawn((
                        PlayerWorld { player: client_entity },
                        Name::new("Player World"),
                        loc,
                        RapierContextEntityLink(
                            q_default_rapier_context
                                .get_single()
                                .expect("The client has no default rapier context!"),
                        ),
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
                block_entities,
            } => {
                if let Some(s_entity) = network_mapping.client_from_server(&server_structure_entity) {
                    if let Ok(mut structure) = q_structure.get_mut(s_entity) {
                        let chunk: Chunk = cosmos_encoder::deserialize(&serialized_chunk).expect("Unable to deserialize chunk from server");
                        let chunk_coords = chunk.chunk_coordinates();

                        structure.set_chunk(chunk);

                        for (_, block_data_entity) in block_entities {
                            info!("New block data -- asking.");
                            client.send_message(
                                NettyChannelClient::Reliable,
                                cosmos_encoder::serialize(&ClientReliableMessages::RequestEntityData { entity: block_data_entity }),
                            );
                        }

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
                    if let Ok(mut structure) = q_structure.get_mut(s_entity) {
                        structure.set_to_empty_chunk(coords);

                        set_chunk_event_writer.send(ChunkInitEvent {
                            coords,
                            structure_entity: s_entity,
                            serialized_block_data: None,
                        });
                    }
                }
            }
            ServerReliableMessages::EntityDespawn { entity: server_entity } => {
                if let Some(entity) = get_entity_identifier_entity_for_despawning(
                    server_entity,
                    &network_mapping,
                    &q_structure_systems,
                    &mut q_inventory,
                    &mut q_structure,
                    &mut evw_block_data_changed,
                ) {
                    if let Some(mut ecmds) = commands.get_entity(entity) {
                        ecmds.insert(NeedsDespawned);
                    }
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
                    if let Ok(mut structure) = q_structure.get_mut(client_ent) {
                        for block_changed in blocks_changed_packet.0 {
                            structure.set_block_at(
                                block_changed.coordinates.coords(),
                                blocks.from_numeric_id(block_changed.block_id),
                                block_changed.block_rotation,
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
                    css::ORANGE_RED.into(),
                ));
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
            ServerReliableMessages::TerrainGenerationShaders {
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

fn get_entity_identifier_entity_for_despawning(
    entity_identifier: ComponentEntityIdentifier,
    network_mapping: &NetworkMapping,
    q_structure_systems: &Query<&StructureSystems, ()>,
    q_inventory: &mut Query<&mut Inventory>,
    q_structure: &mut Query<&mut Structure>,
    block_data_changed: &mut EventWriter<BlockDataChangedEvent>,
) -> Option<Entity> {
    let identifier_entities = match entity_identifier {
        ComponentEntityIdentifier::Entity(entity) => network_mapping.client_from_server(&entity),
        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
            network_mapping.client_from_server(&structure_entity).and_then(|structure_entity| {
                let structure_systems = q_structure_systems.get(structure_entity).ok()?;

                let system_entity = structure_systems.get_system_entity(id)?;

                Some(system_entity)
            })
        }
        ComponentEntityIdentifier::ItemData {
            inventory_entity,
            item_slot,
            server_data_entity,
        } => network_mapping.client_from_server(&inventory_entity).and_then(|inventory_entity| {
            let mut inventory = q_inventory.get_mut(inventory_entity).ok()?;

            let de = inventory.mut_itemstack_at(item_slot as usize).and_then(|x| {
                let de = x.data_entity();
                x.set_data_entity(None);
                de
            });

            // If de is none, the inventory was already synced from the server, so the itemstack has no data pointer. Thus,
            // all we have to do is despawn the data entity.
            if de.is_none() {
                network_mapping.client_from_server(&server_data_entity)
            } else {
                de
            }
        }),
        ComponentEntityIdentifier::BlockData {
            identifier,
            server_data_entity,
        } => network_mapping
            .client_from_server(&identifier.structure_entity)
            .and_then(|structure_entity| {
                let mut structure = q_structure.get_mut(structure_entity).ok()?;

                let bd = structure.block_data(identifier.block.coords());

                if let Some(bd) = bd {
                    // If we have already cleaned up this entity, we don't want to replace the new one.
                    if network_mapping
                        .server_from_client(&bd)
                        .map(|x| x != server_data_entity)
                        .unwrap_or(true)
                    {
                        return None;
                    }

                    structure.set_block_data_entity(identifier.block.coords(), None);

                    block_data_changed.send(BlockDataChangedEvent {
                        block: identifier.block,
                        structure_entity,
                        block_data_entity: None,
                    });
                }

                Some(bd)
            })
            .unwrap_or_else(|| network_mapping.client_from_server(&server_data_entity)),
    };

    if let Some(identifier_entities) = identifier_entities {
        return Some(identifier_entities);
    }

    match entity_identifier {
        ComponentEntityIdentifier::Entity(entity) => {
            warn!(
                "Got entity to despawn, but no valid entity exists for it! ({entity:?}). In the future, this should try again once we receive the correct entity from the server."
            );
            None
        }
        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
            warn!(
                    "Got structure system to despawn, but no valid structure exists for it! ({structure_entity:?}, {id:?}). In the future, this should try again once we receive the correct structure from the server."
            );
            None
        }
        ComponentEntityIdentifier::ItemData {
            inventory_entity,
            item_slot,
            server_data_entity,
        } => {
            warn!(
                "Got itemdata to despawn, but no valid inventory OR itemstack exists for it! ({inventory_entity:?}, {item_slot} {server_data_entity:?}). In the future, this should try again once we receive the correct inventory from the server."
            );
            None
        }
        ComponentEntityIdentifier::BlockData {
            identifier,
            server_data_entity,
        } => {
            warn!(
                "Got block data to despawn, but no valid structure OR block exists for it! ({identifier:?}, {server_data_entity:?}). In the future, this should try again once we receive the correct structure+block from the server."
            );
            None
        }
    }
}

/// Handles any just-added locations that need to sync up to their transforms
fn fix_location(
    mut query: Query<(Entity, &mut Location, Option<&mut Transform>), (Added<Location>, Without<PlayerWorld>, Without<Parent>)>,
    player_worlds: Query<(Entity, &Location), With<PlayerWorld>>,
    mut commands: Commands,
) {
    for (entity, mut location, transform) in query.iter_mut() {
        match player_worlds.get_single() {
            Ok((pw, loc)) => {
                let translation = loc.relative_coords_to(&location);
                if let Some(mut transform) = transform {
                    transform.translation = translation;
                } else {
                    commands.entity(entity).insert((
                        WorldWithin(pw),
                        TransformBundle::from_transform(Transform::from_translation(translation)),
                    ));
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
        .add_systems(
            Update,
            (
                insert_last_rotation,
                update_crosshair.in_set(CrosshairOffsetSet::ApplyCrosshairChanges),
            )
                .after(ClientCreateShipMovementSet::ProcessShipMovement)
                .in_set(NetworkingSystemsSet::Between)
                .after(CursorFlagsSet::ApplyCursorFlagsUpdates)
                .chain(),
        )
        .add_systems(
            Update,
            (
                fix_location,
                client_sync_players
                    .before(ClientReceiveComponents::ClientReceiveComponents)
                    .in_set(NetworkingSystemsSet::ReceiveMessages)
                    .before(CosmosBundleSet::HandleCosmosBundles),
            )
                .run_if(in_state(GameState::Playing).or_else(in_state(GameState::LoadingWorld)))
                .chain(),
        )
        .add_systems(
            Update,
            (
                // Also run first above
                fix_location,
                (
                    lerp_towards,
                    player_changed_parent,
                    sync_transforms_and_locations,
                    handle_child_syncing,
                    add_previous_location,
                )
                    .after(CosmosBundleSet::HandleCosmosBundles)
                    .chain(),
            )
                .in_set(LocationPhysicsSet::DoPhysics)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
}
