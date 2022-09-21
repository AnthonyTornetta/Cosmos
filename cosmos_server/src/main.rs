mod events;
mod server;
mod structure;

use bevy::prelude::*;
use bevy::winit::WinitPlugin;
use bevy_rapier3d::prelude::{Collider, LockedAxes, RigidBody, Velocity};
use bevy_renet::renet::{RenetServer, ServerAuthentication, ServerConfig, ServerEvent};
use bevy_renet::RenetServerPlugin;
use cosmos_core::entities::player::Player;
use cosmos_core::netty::netty::ServerReliableMessages::{
    ChunkData, PlayerCreate, PlayerRemove, StructureCreate, MOTD,
};
use cosmos_core::netty::netty::ServerUnreliableMessages::BulkBodies;
use cosmos_core::netty::netty::{
    server_connection_config, ClientReliableMessages, ClientUnreliableMessages, NettyChannel,
    PROTOCOL_ID,
};
use cosmos_core::netty::netty_rigidbody::NettyRigidBody;
use cosmos_core::physics::structure_physics::{
    listen_for_new_physics_event, listen_for_structure_event,
};
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;
use cosmos_core::structure::events::StructureCreated;
use cosmos_core::structure::planet::planet_builder::TPlanetBuilder;
use cosmos_core::structure::structure::Structure;
use events::blocks::block_events::{self, *};
use noise::Seedable;
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;
use structure::planet::biosphere::grass_biosphere::{self, GrassBiosphere};
use structure::planet::server_planet_builder::ServerPlanetBuilder;

#[derive(Debug, Default)]
pub struct ServerLobby {
    pub players: HashMap<u64, Entity>,
}

#[derive(Debug, Default)]
pub struct NetworkTick(u32);

fn server_sync_bodies(
    mut server: ResMut<RenetServer>,
    mut tick: ResMut<NetworkTick>,
    players: Query<(Entity, &Transform, &Velocity)>,
) {
    let mut bodies = Vec::new();

    for (entity, transform, velocity) in players.iter() {
        bodies.push((entity.clone(), NettyRigidBody::new(&velocity, &transform)));
    }

    tick.0 += 1;

    let sync_message = BulkBodies {
        time_stamp: tick.0,
        bodies,
    };
    let message = bincode::serialize(&sync_message).unwrap();

    server.broadcast_message(NettyChannel::Unreliable.id(), message);
}

#[derive(Default)]
struct ClientTicks {
    ticks: HashMap<u64, Option<u32>>,
}

fn server_listen_messages(
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&mut Transform, &mut Velocity), With<Player>>,
    structure_query: Query<&Structure>,
    mut break_block_event: EventWriter<BlockBreakEvent>,
    mut block_interact_event: EventWriter<BlockInteractEvent>,
    mut place_block_event: EventWriter<BlockPlaceEvent>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannel::Unreliable.id()) {
            let command: ClientUnreliableMessages = bincode::deserialize(&message).unwrap();

            match command {
                ClientUnreliableMessages::PlayerBody { body } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        if let Ok((mut transform, mut velocity)) = players.get_mut(*player_entity) {
                            transform.translation = body.translation.into();
                            transform.rotation = body.rotation.into();
                            velocity.linvel = body.body_vel.linvel.into();
                            velocity.angvel = body.body_vel.angvel.into();
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
                    let structure = structure_query.get(server_entity.clone()).unwrap();

                    for chunk in structure.chunks() {
                        server.send_message(
                            client_id,
                            NettyChannel::Reliable.id(),
                            bincode::serialize(&ChunkData {
                                structure_entity: server_entity.clone(),
                                serialized_chunk: bincode::serialize(chunk).unwrap(),
                            })
                            .unwrap(),
                        );
                    }
                }
                ClientReliableMessages::BreakBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                } => {
                    break_block_event.send(BlockBreakEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                    });
                }
                ClientReliableMessages::PlaceBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                    block_id,
                } => {
                    place_block_event.send(BlockPlaceEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                        block_id,
                    });
                }
                ClientReliableMessages::InteractWithBlock {
                    structure_entity,
                    x,
                    y,
                    z,
                } => {
                    block_interact_event.send(BlockInteractEvent {
                        structure_entity,
                        x,
                        y,
                        z,
                    });
                }
            }
        }
    }
}

fn handle_events_system(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    players: Query<(Entity, &Player, &Transform, &Velocity)>,
    structures_query: Query<(Entity, &Structure, &Transform, &Velocity)>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _user_data) => {
                println!("Client {} connected", id);

                for (entity, player, transform, velocity) in players.iter() {
                    let body = NettyRigidBody::new(&velocity, &transform);

                    let msg = bincode::serialize(&PlayerCreate {
                        entity,
                        id: player.id,
                        body,
                        name: player.name.clone(),
                    })
                    .unwrap();

                    server.send_message(*id, NettyChannel::Reliable.id(), msg);
                }

                let name = "epic nameo";
                let player = Player::new(String::from(name), *id);
                let transform = Transform::from_xyz(0.0, 60.0, 0.0);
                let velocity = Velocity::default();

                let netty_body = NettyRigidBody::new(&velocity, &transform);

                let mut player_entity = commands.spawn();
                player_entity.insert(transform);
                player_entity.insert(LockedAxes::ROTATION_LOCKED);
                player_entity.insert(RigidBody::Dynamic);
                player_entity.insert(velocity);
                player_entity.insert(Collider::capsule_y(0.5, 0.25));
                player_entity.insert(player);

                lobby.players.insert(*id, player_entity.id());

                let msg = bincode::serialize(&PlayerCreate {
                    entity: player_entity.id(),
                    id: *id,
                    name: String::from(name),
                    body: netty_body,
                })
                .unwrap();

                server.send_message(
                    *id,
                    NettyChannel::Reliable.id(),
                    bincode::serialize(&MOTD {
                        motd: "Welcome to the server!".into(),
                    })
                    .unwrap(),
                );

                server.broadcast_message(NettyChannel::Reliable.id(), msg);

                for (entity, structure, transform, velocity) in structures_query.iter() {
                    println!("Sending structure...");

                    server.send_message(
                        *id,
                        NettyChannel::Reliable.id(),
                        bincode::serialize(&StructureCreate {
                            entity: entity.clone(),
                            body: NettyRigidBody::new(velocity, transform),
                            width: structure.chunks_width(),
                            height: structure.chunks_height(),
                            length: structure.chunks_length(),
                        })
                        .unwrap(),
                    );
                }
            }
            ServerEvent::ClientDisconnected(id) => {
                println!("Client {} disconnected", id);

                client_ticks.ticks.remove(id);
                if let Some(player_entity) = lobby.players.remove(&id) {
                    commands.entity(player_entity).despawn();
                }

                let message = bincode::serialize(&PlayerRemove { id: *id }).unwrap();

                server.broadcast_message(NettyChannel::Reliable.id(), message);
            }
        }
    }
}

fn create_structure(mut commands: Commands, mut event_writer: EventWriter<StructureCreated>) {
    let mut entity_cmd = commands.spawn();

    let mut structure = Structure::new(2, 3, 2, entity_cmd.id());

    let builder = ServerPlanetBuilder::new(GrassBiosphere::default());

    builder.insert_planet(
        &mut entity_cmd,
        Transform::from_xyz(0.0, 0.0, 0.0),
        &mut structure,
    );

    entity_cmd.insert(structure);

    event_writer.send(StructureCreated {
        entity: entity_cmd.id(),
    });
}

fn main() {
    let port: u16 = 1337;

    let address: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let socket = UdpSocket::bind(address).unwrap();

    let server_config = ServerConfig::new(20, PROTOCOL_ID, address, ServerAuthentication::Unsecure);
    let connection_config = server_connection_config(); //RenetConnectionConfig::default();
    let cur_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let server = RenetServer::new(cur_time, server_config, connection_config, socket).unwrap();

    let noise = noise::OpenSimplex::default();

    noise.set_seed(
        (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % u32::MAX as u128) as u32,
    );

    let mut app = App::new();

    app.add_plugins(CosmosCorePluginGroup::default())
        .add_plugin(RenetServerPlugin)
        .add_plugin(WinitPlugin::default())
        .insert_resource(ServerLobby::default())
        .insert_resource(NetworkTick(0))
        .insert_resource(ClientTicks::default())
        .insert_resource(server)
        .insert_resource(noise)
        .add_startup_system(create_structure)
        .add_system(server_listen_messages)
        .add_system(server_sync_bodies)
        .add_system(handle_events_system)
        .add_system(listen_for_structure_event)
        .add_system(listen_for_new_physics_event);

    block_events::register(&mut app);

    grass_biosphere::register(&mut app);

    app.run();
}
