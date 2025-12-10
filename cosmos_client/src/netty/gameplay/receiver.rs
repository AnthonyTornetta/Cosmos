//! A receiver + processor for a bunch of network packets.
//!
//! This should eventually be broken up

use std::sync::{Arc, Mutex};

use bevy::{
    color::palettes::css, core_pipeline::oit::OrderIndependentTransparencySettings, post_process::bloom::Bloom, prelude::*,
    render::view::Hdr, window::PrimaryWindow,
};
use bevy_inspector_egui::bevy_egui::PrimaryEguiContext;
use bevy_kira_audio::SpatialAudioReceiver;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::Block,
    ecs::{NeedsDespawned, compute_totally_accurate_global_transform},
    entities::player::{Player, render_distance::RenderDistance},
    events::{
        block_events::{BlockChangedMessage, BlockDataChangedMessage},
        structure::change_pilot_event::ChangePilotMessage,
    },
    inventory::{Inventory, held_item_slot::HeldItemSlot},
    netty::{
        NettyChannelClient, NettyChannelServer,
        client::LocalPlayer,
        client_reliable_messages::ClientReliableMessages,
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages,
        sync::{
            ComponentEntityIdentifier,
            mapping::{Mappable, NetworkMapping, ServerEntity},
        },
        system_sets::NetworkingSystemsSet,
    },
    persistence::LoadingDistance,
    physics::{
        location::{Location, SYSTEM_SECTORS, SetPosition, systems::Anchor},
        player_world::PlayerWorld,
    },
    prelude::Station,
    registry::Registry,
    state::GameState,
    structure::{
        ChunkInitMessage, Structure,
        block_health::events::BlockTakeDamageMessage,
        block_storage::BlockStorer,
        chunk::Chunk,
        dynamic_structure::DynamicStructure,
        full_structure::FullStructure,
        planet::biosphere::BiosphereMarker,
        ship::{Ship, pilot::Pilot},
        systems::{StructureSystems, dock_system::Docked},
    },
};
use renet_steam::SteamClientTransport;

use crate::{
    camera::camera_controller::CameraHelper,
    netty::{
        lobby::{ClientLobby, PlayerInfo},
        steam::User,
    },
    rendering::{CameraPlayerOffset, MainCamera},
    settings::DesiredFov,
    structure::planet::generation::SetTerrainGenData,
    ui::{
        crosshair::{CrosshairOffset, CrosshairOffsetSet},
        message::{HudMessage, HudMessages},
    },
};

#[derive(Component)]
struct LastRotation(Quat);

fn insert_last_rotation(mut commands: Commands, query: Query<(Entity, &GlobalTransform), Or<(Added<Structure>, Changed<Pilot>)>>) {
    for (ent, trans) in query.iter() {
        commands.entity(ent).insert(LastRotation(trans.rotation()));
    }
}

fn update_crosshair(
    mut q_ships: Query<(&Pilot, &mut LastRotation, Option<&Docked>), (With<Ship>,)>,
    local_player_query: Query<(), With<LocalPlayer>>,
    camera_query: Query<(Entity, &Transform, &Camera), With<MainCamera>>,
    mut crosshair_offset: ResMut<CrosshairOffset>,
    primary_query: Query<&Window, With<PrimaryWindow>>,
    q_trans: Query<(&Transform, Option<&ChildOf>)>,
) {
    for (pilot, mut last_rotation, docked) in q_ships.iter_mut() {
        if !local_player_query.contains(pilot.entity) {
            continue;
        }

        let Ok((cam_ent, cam_trans, camera)) = camera_query.single() else {
            return;
        };

        let cam_global_trans = compute_totally_accurate_global_transform(cam_ent, &q_trans).expect("Invalid camera heirarchy.");

        let Ok(primary) = primary_query.single() else {
            return;
        };

        let rot_forward = last_rotation.0.mul_vec3(Vec3::from(cam_trans.forward()));

        if docked.is_some() {
            crosshair_offset.x = 0.0;
            crosshair_offset.y = 0.0;
        } else if let Ok(mut pos_on_screen) = camera.world_to_viewport(&cam_global_trans, rot_forward + cam_global_trans.translation()) {
            pos_on_screen -= Vec2::new(primary.width() / 2.0, primary.height() / 2.0);

            // info!("{} {:?} {}", cam_global_trans.translation(), rot_forward, pos_on_screen);
            crosshair_offset.x += pos_on_screen.x;
            crosshair_offset.y -= pos_on_screen.y;
        }

        last_rotation.0 = cam_global_trans.rotation();
    }
}

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
/// Unused
pub struct NetworkTick(pub u64);

#[derive(Debug, Component, Deref)]
pub(crate) struct LerpTowards(NettyRigidBody);

fn lerp_towards(mut query: Query<(&LerpTowards, &mut Transform, &mut Velocity, &mut Location)>) {
    for (lerp_towards, mut transform, mut velocity, mut location) in query.iter_mut() {
        match lerp_towards.location {
            NettyRigidBodyLocation::Absolute(abs_loc) => {
                let to_location = abs_loc;

                // if to_location.distance_sqrd(&location) > 100.0 {
                location.set_from(&to_location);
                // } else {
                // let lerpped_loc = *location + (location.relative_coords_to(&to_location)) * 0.1;
                //
                // location.set_from(&lerpped_loc);
                // }
            }
            NettyRigidBodyLocation::Relative(rel_trans, _) => {
                // if transform.translation.distance_squared(rel_trans) > 100.0 {
                transform.translation = rel_trans;
                // } else {
                //     transform.translation = transform.translation.lerp(rel_trans, 0.1);
                // }
            }
        };

        transform.rotation = lerp_towards.rotation;
        // transform.rotation.lerp(lerp_towards.rotation, 0.1);

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
    (mut client, transport, mut lobby, mut network_mapping): (
        ResMut<RenetClient>,
        Res<SteamClientTransport>,
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
        MessageWriter<ChunkInitMessage>,
        MessageWriter<BlockChangedMessage>,
        MessageWriter<BlockTakeDamageMessage>,
        MessageWriter<SetTerrainGenData>,
        MessageWriter<BlockDataChangedMessage>,
    ),
    (q_default_rapier_context, query_player, q_structure_systems, mut q_inventory, mut q_structure): (
        Query<Entity, With<DefaultRapierContext>>,
        Query<&Player>,
        Query<&StructureSystems>,
        Query<&mut Inventory>,
        Query<&mut Structure>,
    ),
    q_local_player: Query<(), With<LocalPlayer>>,
    (mut query_body, q_g_trans): (
        Query<
            (
                Option<&mut Location>,
                Option<&mut Transform>,
                Option<&Velocity>,
                Option<&mut NetworkTick>,
                Option<&mut LerpTowards>,
            ),
            Without<LocalPlayer>,
        >,
        Query<&GlobalTransform>,
    ),
    user: Res<User>,
    desired_fov: Res<DesiredFov>,
    q_parent: Query<&ChildOf>,
    blocks: Res<Registry<Block>>,
    mut pilot_change_event_writer: MessageWriter<ChangePilotMessage>,

    mut hud_messages: ResMut<HudMessages>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Unreliable) {
        let msg: ServerUnreliableMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::BulkBodies { bodies, time_stamp } => {
                for (server_entity, body) in bodies.iter() {
                    let Ok(body) = body.map_to_client(&network_mapping) else {
                        continue;
                    };

                    let entity = network_mapping.client_from_server_or_create(server_entity, &mut commands);
                    if let Some((location, transform, velocity, net_tick, lerp_towards)) =
                        query_body.get_mut(entity).ok().map(Some).unwrap_or_else(|| {
                            if !q_local_player.contains(entity) {
                                Some((None, None, None, None, None))
                            } else {
                                None
                            }
                        })
                    {
                        if let Some(mut net_tick) = net_tick {
                            if net_tick.0 >= time_stamp {
                                // Received position packet for previous time, disregard.
                                continue;
                            } else {
                                net_tick.0 = time_stamp;
                            }
                        } else {
                            commands.entity(entity).try_insert(NetworkTick(time_stamp));
                        }

                        if location.is_some() && transform.is_some() && velocity.is_some() {
                            if let Some(mut lerp_towards) = lerp_towards {
                                lerp_towards.0 = body;
                            } else {
                                commands.entity(entity).try_insert(LerpTowards(body));
                            }
                        } else {
                            let loc = match body.location {
                                NettyRigidBodyLocation::Absolute(location) => {
                                    if q_parent.contains(entity) {
                                        info!("Removing parent for {entity:?}");
                                        commands.entity(entity).remove_parent_in_place();
                                    }

                                    location
                                }
                                NettyRigidBodyLocation::Relative(rel_trans, parent_ent) => {
                                    let parent_loc = query_body.get(parent_ent).map(|x| x.0.copied()).unwrap_or(None).unwrap_or_default();

                                    let parent_rot = q_g_trans.get(parent_ent).map(|x| x.rotation()).unwrap_or_default();

                                    if let Ok(parent) = q_parent.get(entity) {
                                        if parent.parent() != parent_ent {
                                            commands.entity(entity).set_parent_in_place(parent_ent);
                                        }
                                    } else {
                                        commands.entity(entity).set_parent_in_place(parent_ent);
                                    }

                                    parent_loc + parent_rot * rel_trans
                                }
                            };

                            if let Ok(mut ecmds) = commands.get_entity(entity) {
                                ecmds.try_insert((
                                    loc,
                                    SetPosition::Transform,
                                    Transform::from_rotation(body.rotation),
                                    body.create_velocity(),
                                    LerpTowards(body),
                                ));
                            }
                        }
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
                parent: server_parent_entity,
                name,
                render_distance: _,
            } => {
                // Prevents creation of duplicate players
                if lobby.players.contains_key(&id) {
                    warn!("DUPLICATE PLAYER RECEIVED {id}");
                    continue;
                }

                let Ok(body) = body.map_to_client(&network_mapping) else {
                    continue;
                };

                info!("Player {} ({}) connected!", name.as_str(), id);

                let parent_entity = server_parent_entity.map(|x| network_mapping.client_from_server_or_create(&x, &mut commands));

                // The player entity may have already been created if some of their components were already synced.
                let mut entity_cmds = if let Some(player_entity) = network_mapping.client_from_server(&server_entity) {
                    commands.entity(player_entity)
                } else {
                    commands.spawn_empty()
                };

                let loc = match body.location {
                    NettyRigidBodyLocation::Absolute(location) => location,
                    NettyRigidBodyLocation::Relative(rel_trans, entity) => {
                        let parent_loc = query_body.get(entity).map(|x| x.0.copied()).ok().flatten().unwrap_or_default();

                        parent_loc + rel_trans
                    }
                };

                entity_cmds.insert((
                    SetPosition::Transform,
                    Transform::from_rotation(body.rotation),
                    loc,
                    body.create_velocity(),
                    Player::new(name, id),
                    ServerEntity(server_entity),
                ));

                info!("Got player @ {loc:?}");

                let client_entity = entity_cmds.id();

                if let Some(parent_entity) = parent_entity {
                    entity_cmds.set_parent_in_place(parent_entity);
                }

                let player_info = PlayerInfo {
                    server_entity,
                    client_entity,
                };

                lobby.players.insert(id, player_info);
                network_mapping.add_mapping(client_entity, server_entity);

                let camera_offset = Vec3::new(0.0, 0.75, 0.0);

                let client_id = transport.client_id(user.client());
                if client_id == id {
                    entity_cmds
                        .insert((
                            LocalPlayer,
                            Anchor,
                            HeldItemSlot::new(0).unwrap(),
                            RenderDistance::default(),
                            CameraPlayerOffset(camera_offset),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Hdr,
                                Camera { ..Default::default() },
                                Transform::from_translation(camera_offset),
                                Projection::from(PerspectiveProjection {
                                    fov: (desired_fov.0 / 180.0) * std::f32::consts::PI,
                                    ..default()
                                }),
                                Camera3d::default(),
                                Bloom { ..Default::default() },
                                CameraHelper::default(),
                                Name::new("Main Camera"),
                                OrderIndependentTransparencySettings::default(),
                                MainCamera,
                                // https://github.com/jakobhellermann/bevy-inspector-egui/issues/286
                                PrimaryEguiContext,
                                IsDefaultUiCamera,
                                Msaa::Off,
                                SpatialAudioReceiver,
                            ));
                        });

                    let physics_world_ent = q_default_rapier_context
                        .single()
                        .expect("The client has no default rapier context!");

                    commands.entity(physics_world_ent).insert((
                        PlayerWorld { player: client_entity },
                        Name::new("Player World"),
                        loc,
                        RapierContextEntityLink(physics_world_ent),
                    ));
                }
            }
            ServerReliableMessages::PlayerRemove { id } => {
                if let Some(PlayerInfo {
                    client_entity,
                    server_entity: _,
                }) = lobby.players.remove(&id)
                    && let Ok(mut ecmds) = commands.get_entity(client_entity)
                {
                    if let Ok(player) = query_player.get(client_entity) {
                        info!("Player {} ({id}) disconnected", player.name());
                    }

                    ecmds.insert(NeedsDespawned);
                }
            }
            // This could cause issues in the future if a client receives a planet's position first then this packet.
            // Please restructure this + the ship to use the new requesting system.
            ServerReliableMessages::Planet {
                entity: server_entity,
                dimensions,
                planet,
                biosphere,
            } => {
                let entity = network_mapping.client_from_server_or_create(&server_entity, &mut commands);

                let mut entity_cmds = commands.entity(entity);
                let structure = Structure::Dynamic(DynamicStructure::new(dimensions));

                entity_cmds.insert((structure, planet, BiosphereMarker::new(biosphere)));
            }
            ServerReliableMessages::NumberOfChunks {
                entity: server_entity,
                chunks_needed,
            } => {
                let Some(entity) = network_mapping.client_from_server(&server_entity) else {
                    continue;
                };

                if let Ok(mut ecmds) = commands.get_entity(entity) {
                    ecmds.insert(chunks_needed);
                }
            }
            ServerReliableMessages::Ship {
                entity: server_entity,
                dimensions,
            } => {
                let entity = network_mapping.client_from_server_or_create(&server_entity, &mut commands);

                let mut entity_cmds = commands.entity(entity);
                let structure = Structure::Full(FullStructure::new(dimensions));

                entity_cmds.insert((structure, Ship));

                client.send_message(
                    NettyChannelClient::Reliable,
                    cosmos_encoder::serialize(&ClientReliableMessages::PilotQuery {
                        ship_entity: server_entity,
                    }),
                );
            }
            ServerReliableMessages::Station {
                entity: server_entity,
                dimensions,
            } => {
                let entity = network_mapping.client_from_server_or_create(&server_entity, &mut commands);

                let mut entity_cmds = commands.entity(entity);
                let structure = Structure::Full(FullStructure::new(dimensions));

                entity_cmds.insert((structure, Station));
            }
            ServerReliableMessages::ChunkData {
                structure_entity: server_structure_entity,
                serialized_chunk,
                serialized_block_data,
                block_entities,
            } => {
                if let Some(s_entity) = network_mapping.client_from_server(&server_structure_entity)
                    && let Ok(mut structure) = q_structure.get_mut(s_entity)
                {
                    let mut chunk: Chunk = cosmos_encoder::deserialize(&serialized_chunk).expect("Unable to deserialize chunk from server");
                    let chunk_coords = chunk.chunk_coordinates();

                    for ((block_id, coords), block_data_entity) in block_entities {
                        if let Some(client_ent) = network_mapping.client_from_server(&block_data_entity) {
                            let here_id = chunk.block_at(coords);
                            if here_id == block_id {
                                chunk.set_block_data_entity(coords, Some(client_ent));
                            } else {
                                error!(
                                    "Blocks didn't match up for block data! This may cause a block to have missing data. Block data block id: {block_id}; block here id: {here_id}."
                                );
                            }
                        }
                    }

                    structure.set_chunk(chunk);

                    set_chunk_event_writer.write(ChunkInitMessage {
                        coords: chunk_coords,
                        structure_entity: s_entity,
                        serialized_block_data: serialized_block_data.map(|x| Arc::new(Mutex::new(x))),
                    });
                }
            }
            ServerReliableMessages::EmptyChunk { structure_entity, coords } => {
                if let Some(s_entity) = network_mapping.client_from_server(&structure_entity)
                    && let Ok(mut structure) = q_structure.get_mut(s_entity)
                {
                    structure.set_to_empty_chunk(coords);

                    set_chunk_event_writer.write(ChunkInitMessage {
                        coords,
                        structure_entity: s_entity,
                        serialized_block_data: None,
                    });
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
                ) && let Ok(mut ecmds) = commands.get_entity(entity)
                {
                    ecmds.insert(NeedsDespawned);
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
                if let Some(client_ent) = network_mapping.client_from_server(&structure_entity)
                    && let Ok(mut structure) = q_structure.get_mut(client_ent)
                {
                    for block_changed in blocks_changed_packet.0 {
                        structure.set_block_and_info_at(
                            block_changed.coordinates.coords(),
                            blocks.from_numeric_id(block_changed.block_id),
                            block_changed.block_info,
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

                pilot_change_event_writer.write(ChangePilotMessage {
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
                if let Some(player_entity) = network_mapping.client_from_server(&player_entity)
                    && let Ok(mut ecmds) = commands.get_entity(player_entity)
                {
                    ecmds.remove_parent_in_place();
                }
            }
            ServerReliableMessages::PlayerJoinShip {
                player_entity,
                ship_entity,
            } => {
                let Some(player_entity) = network_mapping.client_from_server(&player_entity) else {
                    continue;
                };

                let Ok(mut ecmds) = commands.get_entity(player_entity) else {
                    continue;
                };

                let Some(ship_entity) = network_mapping.client_from_server(&ship_entity) else {
                    continue;
                };

                ecmds.set_parent_in_place(ship_entity);
            }
            ServerReliableMessages::InvalidReactor { reason } => {
                hud_messages.display_message(HudMessage::with_colored_string(
                    format!("Invalid reactor setup: {reason}"),
                    css::ORANGE_RED.into(),
                ));
            }
            ServerReliableMessages::BlockHealthChange { changes } => {
                take_damage_event_writer.write_batch(changes.into_iter().filter_map(|ev| {
                    network_mapping
                        .client_from_server(&ev.structure_entity)
                        .map(|structure_entity| BlockTakeDamageMessage {
                            structure_entity,
                            block: ev.block,
                            new_health: ev.new_health,
                            causer: ev.causer.and_then(|x| network_mapping.client_from_server(&x)),
                        })
                }));
            }
            ServerReliableMessages::TerrainGenerationShaders {
                shaders,
                permutation_table,
            } => {
                set_terrain_data_ev_writer.write(SetTerrainGenData {
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
    block_data_changed: &mut MessageWriter<BlockDataChangedMessage>,
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
            mut identifier,
            server_data_entity,
        } => network_mapping
            .client_from_server(&identifier.block.structure())
            .and_then(|structure_entity| {
                identifier.block.set_structure(structure_entity);

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

                    block_data_changed.write(BlockDataChangedMessage {
                        block: identifier.block,
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

//
// /// Fixes oddities that happen when changing parent of player
// fn player_changed_parent(
//     q_parent: Query<(&GlobalTransform, &Location)>,
//     mut q_local_player: Query<(&mut Transform, &Location, &ChildOf), (Changed<ChildOf>, With<LocalPlayer>)>,
// ) {
//     let Ok((mut player_trans, player_loc, parent)) = q_local_player.single_mut() else {
//         return;
//     };
//
//     let Ok((parent_trans, parent_loc)) = q_parent.get(parent.parent()) else {
//         return;
//     };
//
//     // Because the player's translation is always 0, 0, 0 we need to adjust it so the player is put into the
//     // right spot in its parent.
//     player_trans.translation = Quat::from_affine3(&parent_trans.affine())
//         .inverse()
//         .mul_vec3((*player_loc - *parent_loc).absolute_coords_f32());
// }

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            insert_last_rotation,
            update_crosshair.in_set(CrosshairOffsetSet::ApplyCrosshairChanges),
        )
            .chain(),
    )
    .add_systems(
        FixedUpdate,
        (client_sync_players, lerp_towards)
            .chain()
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    );
    // .add_systems(
    //     FixedUpdate,
    //lerp_towards
    //         .after(FixedUpdateSet::NettyReceive)
    //         .before(FixedUpdateSet::Main)
    //         .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    // );
}
