use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::{RenetServer, ServerEvent};
use cosmos_core::inventory::Inventory;
use cosmos_core::item::Item;
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::registry::Registry;
use cosmos_core::structure::planet::Planet;
use cosmos_core::structure::ship::Ship;
use cosmos_core::structure::Structure;
use cosmos_core::{
    entities::player::Player,
    netty::{netty_rigidbody::NettyRigidBody, NettyChannel},
};
use renet_visualizer::RenetServerVisualizer;

use crate::netty::network_helpers::{ClientTicks, ServerLobby};

fn generate_player_inventory(items: &Registry<Item>) -> Inventory {
    let mut inventory = Inventory::new(9 * 1);

    inventory.insert_at(
        0,
        items.from_id("cosmos:stone").expect("Stone item to exist"),
        64,
    );

    inventory.insert_at(
        1,
        items.from_id("cosmos:dirt").expect("Dirt item to exist"),
        64,
    );

    inventory.insert_at(
        2,
        items.from_id("cosmos:grass").expect("Grass item to exist"),
        64,
    );

    inventory.insert_at(
        3,
        items
            .from_id("cosmos:thruster")
            .expect("Thruster item to exist"),
        64,
    );

    inventory.insert_at(
        4,
        items
            .from_id("cosmos:laser_cannon")
            .expect("Laser cannon item to exist"),
        64,
    );

    inventory.insert_at(
        5,
        items
            .from_id("cosmos:reactor")
            .expect("Reactor cannon item to exist"),
        64,
    );

    inventory.insert_at(
        6,
        items
            .from_id("cosmos:energy_cell")
            .expect("Energy cell item to exist"),
        64,
    );

    inventory.insert_at(
        7,
        items
            .from_id("cosmos:ship_hull")
            .expect("Ship hull item to exist"),
        999,
    );

    inventory
}

use crate::state::GameState;

fn handle_events_system(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    players: Query<(Entity, &Player, &Transform, &Velocity, &Inventory)>,
    structure_type: Query<(Option<&Ship>, Option<&Planet>)>,
    structures_query: Query<(Entity, &Structure, &Transform, &Velocity)>,
    items: Res<Registry<Item>>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _user_data) => {
                println!("Client {id} connected");
                visualizer.add_client(*id);

                for (entity, player, transform, velocity, inventory) in players.iter() {
                    let body = NettyRigidBody::new(velocity, transform);

                    let msg = bincode::serialize(&ServerReliableMessages::PlayerCreate {
                        entity,
                        id: player.id,
                        body,
                        name: player.name.clone(),
                        inventory_serialized: bincode::serialize(inventory).unwrap(),
                    })
                    .unwrap();

                    server.send_message(*id, NettyChannel::Reliable.id(), msg);
                }

                let name = "epic nameo";
                let player = Player::new(String::from(name), *id);
                let transform = Transform::from_xyz(0.0, 60.0, 0.0);
                let velocity = Velocity::default();
                let inventory = generate_player_inventory(&items);

                let netty_body = NettyRigidBody::new(&velocity, &transform);

                let inventory_serialized = bincode::serialize(&inventory).unwrap();

                let mut player_entity = commands.spawn(transform);
                player_entity
                    .insert(LockedAxes::ROTATION_LOCKED)
                    .insert(RigidBody::Dynamic)
                    .insert(velocity)
                    .insert(Collider::capsule_y(0.5, 0.25))
                    .insert(player)
                    .insert(ReadMassProperties::default())
                    .insert(inventory);

                lobby.players.insert(*id, player_entity.id());

                let msg = bincode::serialize(&ServerReliableMessages::PlayerCreate {
                    entity: player_entity.id(),
                    id: *id,
                    name: String::from(name),
                    body: netty_body,
                    inventory_serialized,
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
                                width: structure.chunks_width() as u32,
                                height: structure.chunks_height() as u32,
                                length: structure.chunks_length() as u32,
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
                                width: structure.chunks_width() as u32,
                                height: structure.chunks_height() as u32,
                                length: structure.chunks_length() as u32,
                            })
                            .unwrap(),
                        );
                    }
                }
            }
            ServerEvent::ClientDisconnected(id) => {
                println!("Client {id} disconnected");
                visualizer.remove_client(*id);
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
    app.add_system_set(SystemSet::on_update(GameState::Playing).with_system(handle_events_system));
}
