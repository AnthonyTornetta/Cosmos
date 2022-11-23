use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::{RenetServer, ServerEvent};
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::structure::planet::planet::Planet;
use cosmos_core::structure::ship::ship::Ship;
use cosmos_core::structure::structure::Structure;
use cosmos_core::{
    entities::player::Player,
    netty::{netty_rigidbody::NettyRigidBody, NettyChannel},
};

use crate::netty::network_helpers::{ClientTicks, ServerLobby};

fn handle_events_system(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    players: Query<(Entity, &Player, &Transform, &Velocity)>,
    structure_type: Query<(Option<&Ship>, Option<&Planet>)>,
    structures_query: Query<(Entity, &Structure, &Transform, &Velocity)>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _user_data) => {
                println!("Client {} connected", id);

                for (entity, player, transform, velocity) in players.iter() {
                    let body = NettyRigidBody::new(velocity, transform);

                    let msg = bincode::serialize(&ServerReliableMessages::PlayerCreate {
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
                let transform = Transform::from_xyz(0.0, 180.0, 0.0);
                let velocity = Velocity::default();

                let netty_body = NettyRigidBody::new(&velocity, &transform);

                let mut player_entity = commands.spawn(transform);
                player_entity.insert(LockedAxes::ROTATION_LOCKED);
                player_entity.insert(RigidBody::Dynamic);
                player_entity.insert(velocity);
                player_entity.insert(Collider::capsule_y(0.5, 0.25));
                player_entity.insert(player);

                lobby.players.insert(*id, player_entity.id());

                let msg = bincode::serialize(&ServerReliableMessages::PlayerCreate {
                    entity: player_entity.id(),
                    id: *id,
                    name: String::from(name),
                    body: netty_body,
                })
                .unwrap();

                server.send_message(
                    *id,
                    NettyChannel::Reliable.id(),
                    bincode::serialize(&ServerReliableMessages::MOTD {
                        motd: "Welcome to the server!".into(),
                    })
                    .unwrap(),
                );

                server.broadcast_message(NettyChannel::Reliable.id(), msg);

                for (entity, structure, transform, velocity) in structures_query.iter() {
                    println!("Sending structure...");

                    let (ship, planet) = structure_type.get(entity).unwrap();

                    if planet.is_some() {
                        server.send_message(
                            *id,
                            NettyChannel::Reliable.id(),
                            bincode::serialize(&ServerReliableMessages::PlanetCreate {
                                entity,
                                body: NettyRigidBody::new(velocity, transform),
                                width: structure.chunks_width(),
                                height: structure.chunks_height(),
                                length: structure.chunks_length(),
                            })
                            .unwrap(),
                        );
                    } else if ship.is_some() {
                        server.send_message(
                            *id,
                            NettyChannel::Reliable.id(),
                            bincode::serialize(&ServerReliableMessages::ShipCreate {
                                entity,
                                body: NettyRigidBody::new(velocity, transform),
                                width: structure.chunks_width(),
                                height: structure.chunks_height(),
                                length: structure.chunks_length(),
                            })
                            .unwrap(),
                        );
                    }
                }
            }
            ServerEvent::ClientDisconnected(id) => {
                println!("Client {} disconnected", id);

                client_ticks.ticks.remove(id);

                if let Some(player_entity) = lobby.players.remove(id) {
                    commands.entity(player_entity).despawn();
                }
                let message =
                    bincode::serialize(&ServerReliableMessages::PlayerRemove { id: *id }).unwrap();

                server.broadcast_message(NettyChannel::Reliable.id(), message);
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(handle_events_system);
}
