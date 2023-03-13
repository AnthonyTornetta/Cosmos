use bevy::{
    core_pipeline::bloom::BloomSettings, prelude::*, render::camera::Projection,
    window::PrimaryWindow,
};
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
    physics::{
        location::{bubble_down_locations, Location},
        player_world::PlayerWorld,
    },
    registry::Registry,
    structure::{
        chunk::Chunk,
        planet::planet_builder::TPlanetBuilder,
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

fn client_sync_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut set_chunk_event_writer: EventWriter<ChunkInitEvent>,
    mut block_change_event_writer: EventWriter<BlockChangedEvent>,
    query_player: Query<&Player>,
    mut query_body: Query<(&mut Location, &mut Transform, &mut Velocity), Without<LocalPlayer>>,
    mut query_structure: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut pilot_change_event_writer: EventWriter<ChangePilotEvent>,
    mut set_ship_movement_event: EventWriter<SetShipMovementEvent>,
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(NettyChannel::Unreliable.id()) {
        let msg: ServerUnreliableMessages = bincode::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::BulkBodies {
                bodies,
                time_stamp: _,
            } => {
                for (server_entity, body) in bodies.iter() {
                    if let Some(entity) = network_mapping.client_from_server(server_entity) {
                        if let Ok((mut location, mut transform, mut velocity)) =
                            query_body.get_mut(*entity)
                        {
                            location.set_from(&body.location);
                            transform.rotation = body.rotation;

                            velocity.linvel = body.body_vel.linvel.into();
                            velocity.angvel = body.body_vel.angvel.into();
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
                mut body,
                id,
                entity: server_entity,
                name,
                inventory_serialized,
            } => {
                println!("Player {} ({}) connected!", name.as_str(), id);

                let mut entity_cmds = commands.spawn_empty();

                let inventory: Inventory = bincode::deserialize(&inventory_serialized).unwrap();

                // This should be set via the server, but just in case,
                // this will avoid any position mismatching
                body.location.last_transform_loc = body.location.local;

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
                network_mapping.add_mapping(&client_entity, &server_entity);

                if client_id == id {
                    entity_cmds
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

                    commands.spawn((
                        PlayerWorld {
                            player: client_entity,
                        },
                        body.location,
                        BodyWorld {
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
                    let entity = commands.entity(client_entity);

                    let name = query_player.get(client_entity).unwrap().name.clone();
                    entity.despawn_recursive();
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
                let mut entity_cmds = commands.spawn_empty();
                let mut structure =
                    Structure::new(width as usize, height as usize, length as usize);

                let builder = ClientPlanetBuilder::default();
                builder.insert_planet(&mut entity_cmds, body.location, &mut structure);

                entity_cmds.insert(structure).insert(NeedsPopulated);

                let entity = entity_cmds.id();

                network_mapping.add_mapping(&entity, &server_entity);

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

                entity_cmds.insert(structure);

                let entity = entity_cmds.id();

                network_mapping.add_mapping(&entity, &server_entity);

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

                set_chunk_event_writer.send(ChunkInitEvent {
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
        }
    }
}

// fn sync_transforms_and_locations(
//     mut trans_query_no_parent: Query<
//         (&mut Transform, &mut Location),
//         (Without<PlayerWorld>, Without<Parent>),
//     >,
//     mut trans_query_with_parent: Query<
//         (&mut Transform, &mut Location),
//         (Without<PlayerWorld>, With<Parent>),
//     >,
//     parent_query: Query<&Parent>,
//     player_query: Query<Entity, With<LocalPlayer>>,
//     // mut player_trans_query: Query<
//     //     (&mut Transform, &mut Location),
//     //     (Without<PlayerWorld>, With<LocalPlayer>),
//     // >,
//     mut world_query: Query<&mut Location, With<PlayerWorld>>,
// ) {
//     for (transform, mut location) in trans_query_no_parent.iter_mut() {
//         location.apply_updates(transform.translation);
//     }
//     for (transform, mut location) in trans_query_with_parent.iter_mut() {
//         location.apply_updates(transform.translation);
//     }

//     if let Ok(mut world_location) = world_query.get_single_mut() {
//         let mut player_entity = player_query.single();

//         while let Ok(parent) = parent_query.get(player_entity) {
//             let parent_entity = parent.get();
//             if trans_query_no_parent.contains(parent_entity) {
//                 player_entity = parent.get();
//             } else {
//                 break;
//             }
//         }

//         let (_, player_location) = trans_query_no_parent
//             .get(player_entity)
//             .or_else(|_| trans_query_with_parent.get(player_entity))
//             .expect("The above loop guarantees this is valid");

//         world_location.set_from(&player_location);
//         world_location.last_transform_loc = Vec3::ZERO;

//         for (mut transform, mut location) in trans_query_no_parent.iter_mut() {
//             let translation = world_location.relative_coords_to(&location);
//             println!("Relative coords: {}", translation);

//             transform.translation = translation;
//             location.last_transform_loc = translation;
//         }

//         // let (mut player_transform, mut player_location) = player_trans_query.single_mut();

//         // player_location.apply_updates(player_transform.translation);

//         // world_location.set_from(&player_location);

//         // player_transform.translation = world_location.relative_coords_to(&player_location);
//         // player_location.last_transform_loc = player_transform.translation;

//         // for (mut transform, mut location) in trans_query.iter_mut() {
//         //     location.apply_updates(transform.translation);
//         //     transform.translation = world_location.relative_coords_to(&location);
//         //     location.last_transform_loc = transform.translation;
//         // }
//     }
// }

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

        // println!("Player loc: {location}");

        // Update transforms of objects within this world.
        for (mut transform, mut location) in trans_query_no_parent.iter_mut() {
            let trans = world_location.relative_coords_to(&location);
            // println!("Trans: {trans}");
            transform.translation = trans;
            location.last_transform_loc = trans;
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(
        client_sync_players
            .run_if(in_state(GameState::Playing))
            .run_if(in_state(GameState::LoadingWorld)),
    )
    .add_systems(
        (
            update_crosshair,
            insert_last_rotation,
            sync_transforms_and_locations.after(client_sync_players),
            bubble_down_locations.after(sync_transforms_and_locations),
        )
            .in_set(OnUpdate(GameState::Playing)),
    );
}
