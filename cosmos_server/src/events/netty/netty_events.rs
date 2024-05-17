//! Handles client connecting and disconnecting

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::transport::NetcodeServerTransport;
use bevy_renet::renet::{ClientId, RenetServer, ServerEvent};
use cosmos_core::economy::Credits;
use cosmos_core::ecs::NeedsDespawned;
use cosmos_core::entities::player::render_distance::RenderDistance;
use cosmos_core::inventory::itemstack::{ItemShouldHaveData, ItemStackSystemSet};
use cosmos_core::inventory::Inventory;
use cosmos_core::item::Item;
use cosmos_core::netty::netty_rigidbody::NettyRigidBodyLocation;
use cosmos_core::netty::server::ServerLobby;
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::netty::sync::server_entity_syncing::RequestedEntityEvent;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::netty::{cosmos_encoder, NettyChannelServer};
use cosmos_core::persistence::LoadingDistance;
use cosmos_core::physics::location::{Location, Sector};
use cosmos_core::physics::player_world::WorldWithin;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::Registry;
use cosmos_core::structure::chunk::CHUNK_DIMENSIONSF;
use cosmos_core::{entities::player::Player, netty::netty_rigidbody::NettyRigidBody};
use renet_visualizer::RenetServerVisualizer;

use crate::entities::player::PlayerLooking;
use crate::netty::network_helpers::ClientTicks;
use crate::physics::assign_player_world;
use crate::state::GameState;

fn generate_player_inventory(
    inventory_entity: Entity,
    items: &Registry<Item>,
    commands: &mut Commands,
    has_data: &ItemShouldHaveData,
) -> Inventory {
    let mut inventory = Inventory::new("Inventory", 9 * 10, Some(0..9), inventory_entity);

    for item in items.iter().rev().filter(|item| item.unlocalized_name() != "cosmos:air") {
        inventory.insert_item(item, item.max_stack_size(), commands, has_data);
    }

    inventory
}

#[derive(Event, Debug)]
/// Sent whenever a player just connected
pub struct PlayerConnectedEvent {
    /// The player's entity
    pub player_entity: Entity,
    /// Player's client id
    pub client_id: ClientId,
}

fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    q_players: Query<(
        Entity,
        &Player,
        &Transform,
        &Location,
        &Velocity,
        &Inventory,
        &RenderDistance,
        &Credits,
    )>,
    player_worlds: Query<(&Location, &WorldWithin, &PhysicsWorld), (With<Player>, Without<Parent>)>,
    items: Res<Registry<Item>>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    mut rapier_context: ResMut<RapierContext>,
    mut requested_entity: EventWriter<RequestedEntityEvent>,
    mut player_join_ev_writer: EventWriter<PlayerConnectedEvent>,
    needs_data: Res<ItemShouldHaveData>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client {client_id} connected");
                visualizer.add_client(client_id);

                for (entity, player, transform, location, velocity, inventory, render_distance, credits) in q_players.iter() {
                    let body = NettyRigidBody::new(Some(*velocity), transform.rotation, NettyRigidBodyLocation::Absolute(*location));

                    let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
                        entity,
                        id: player.id(),
                        body,
                        name: player.name().clone(),
                        inventory_serialized: cosmos_encoder::serialize(inventory),
                        render_distance: Some(*render_distance),
                        credits: *credits,
                    });

                    server.send_message(client_id, NettyChannelServer::Reliable, msg);
                    requested_entity.send(RequestedEntityEvent { client_id, entity });
                }

                let Some(user_data) = transport.user_data(client_id) else {
                    warn!("Unable to get user data!");
                    continue;
                };
                let Ok(name) = bincode::deserialize::<String>(user_data.as_slice()) else {
                    warn!("Unable to deserialize name!");
                    continue;
                };

                let player_entity = commands.spawn_empty().id();

                let player = Player::new(name.clone(), client_id);
                let starting_pos = Vec3::new(0.0, CHUNK_DIMENSIONSF * 70.0 / 2.0, 0.0);
                let location = Location::new(starting_pos, Sector::new(25, 25, 25));
                let velocity = Velocity::default();
                let inventory = generate_player_inventory(player_entity, &items, &mut commands, &needs_data);

                let netty_body = NettyRigidBody::new(Some(velocity), Quat::IDENTITY, NettyRigidBodyLocation::Absolute(location));

                let inventory_serialized = cosmos_encoder::serialize(&inventory);

                let credits = Credits::new(1_000_000);

                commands.entity(player_entity).insert((
                    location,
                    LockedAxes::ROTATION_LOCKED,
                    RigidBody::Dynamic,
                    velocity,
                    Collider::capsule_y(0.65, 0.25),
                    player,
                    ReadMassProperties::default(),
                    inventory,
                    PlayerLooking { rotation: Quat::IDENTITY },
                    LoadingDistance::new(2, 9999),
                    ActiveEvents::COLLISION_EVENTS,
                    Name::new(format!("Player ({name})")),
                    credits,
                ));

                lobby.add_player(client_id, player_entity);

                assign_player_world(&player_worlds, player_entity, &location, &mut commands, &mut rapier_context);

                let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
                    entity: player_entity,
                    id: client_id,
                    name,
                    body: netty_body,
                    inventory_serialized,
                    render_distance: None,
                    credits,
                });

                server.send_message(
                    client_id,
                    NettyChannelServer::Reliable,
                    cosmos_encoder::serialize(&ServerReliableMessages::MOTD {
                        motd: "Welcome to the server!".into(),
                    }),
                );

                server.broadcast_message(NettyChannelServer::Reliable, msg);

                player_join_ev_writer.send(PlayerConnectedEvent { player_entity, client_id });
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Client {client_id} disconnected: {reason}");
                visualizer.remove_client(*client_id);
                client_ticks.ticks.remove(client_id);

                if let Some(player_entity) = lobby.remove_player(*client_id) {
                    commands.entity(player_entity).insert(NeedsDespawned);
                }

                let message = cosmos_encoder::serialize(&ServerReliableMessages::PlayerRemove { id: *client_id });

                server.broadcast_message(NettyChannelServer::Reliable, message);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        handle_server_events
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .before(ItemStackSystemSet::CreateDataEntity),
    )
    .add_event::<PlayerConnectedEvent>();
}
