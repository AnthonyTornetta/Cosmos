//! Handles client connecting and disconnecting

use std::fs;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet2::renet2::transport::NetcodeServerTransport;
use bevy_renet2::renet2::{ClientId, RenetServer, ServerEvent};
use cosmos_core::economy::Credits;
use cosmos_core::ecs::NeedsDespawned;
use cosmos_core::entities::player::creative::Creative;
use cosmos_core::entities::player::render_distance::RenderDistance;
use cosmos_core::inventory::itemstack::ItemShouldHaveData;
use cosmos_core::inventory::Inventory;
use cosmos_core::item::Item;
use cosmos_core::netty::netty_rigidbody::NettyRigidBodyLocation;
use cosmos_core::netty::server::ServerLobby;
use cosmos_core::netty::server_reliable_messages::ServerReliableMessages;
use cosmos_core::netty::sync::registry::server::SyncRegistriesEvent;
use cosmos_core::netty::sync::server_entity_syncing::RequestedEntityEvent;
use cosmos_core::netty::{cosmos_encoder, NettyChannelServer};
use cosmos_core::persistence::LoadingDistance;
use cosmos_core::physics::location::{Location, Sector};
use cosmos_core::physics::player_world::WorldWithin;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::Registry;
use cosmos_core::{entities::player::Player, netty::netty_rigidbody::NettyRigidBody};
use renet2_visualizer::RenetServerVisualizer;
use serde::{Deserialize, Serialize};

use crate::entities::player::PlayerLooking;
use crate::netty::network_helpers::ClientTicks;
use crate::physics::assign_player_world;
use crate::settings::ServerSettings;

#[derive(Debug, Serialize, Deserialize)]
struct KitEntry {
    slot: u32,
    item: String,
    quantity: u16,
}

fn fill_inventory_from_kit(
    kit_name: &str,
    inventory: &mut Inventory,
    items: &Registry<Item>,
    commands: &mut Commands,
    needs_data: &ItemShouldHaveData,
) {
    let Ok(kit) = fs::read_to_string(format!("assets/cosmos/kits/{kit_name}.json")) else {
        error!("Missing kit - {kit_name}");
        return;
    };

    let kit = serde_json::from_str::<Vec<KitEntry>>(&kit).map(|x| Some(x)).unwrap_or_else(|e| {
        error!("{e}");
        None
    });

    let Some(kit) = kit else {
        error!("Invalid kit file - {kit_name}");
        return;
    };

    for entry in kit {
        let Some(item) = items.from_id(&entry.item) else {
            error!("Missing item {} in kit {kit_name}", entry.item);
            continue;
        };

        if entry.slot as usize >= inventory.len() {
            error!("Slot {} in kit {kit_name} out of inventory bounds!", entry.slot);
            continue;
        }

        inventory.insert_item_at(entry.slot as usize, item, entry.quantity, commands, needs_data);
    }
}

fn generate_player_inventory(
    inventory_entity: Entity,
    items: &Registry<Item>,
    commands: &mut Commands,
    has_data: &ItemShouldHaveData,
    creative: bool,
) -> Inventory {
    let mut inventory = Inventory::new("Inventory", 9 * 16, Some(0..9), inventory_entity);

    if creative {
        for item in items.iter().rev().filter(|item| item.unlocalized_name() != "cosmos:air") {
            inventory.insert_item(item, item.max_stack_size(), commands, has_data);
        }
    } else {
        fill_inventory_from_kit("starter", &mut inventory, items, commands, has_data);
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

pub(super) fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    mut server_events: EventReader<ServerEvent>,
    mut lobby: ResMut<ServerLobby>,
    mut client_ticks: ResMut<ClientTicks>,
    q_players: Query<(Entity, &Player, &Transform, &Location, &Velocity, &RenderDistance)>,
    player_worlds: Query<(&Location, &WorldWithin, &RapierContextEntityLink), (With<Player>, Without<Parent>)>,
    items: Res<Registry<Item>>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    mut requested_entity: EventWriter<RequestedEntityEvent>,
    mut evw_player_join: EventWriter<PlayerConnectedEvent>,
    mut evw_sync_registries: EventWriter<SyncRegistriesEvent>,
    needs_data: Res<ItemShouldHaveData>,
    server_settings: Res<ServerSettings>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let client_id = *client_id;
                info!("Client {client_id} connected");
                visualizer.add_client(client_id);

                for (entity, player, transform, location, velocity, render_distance) in q_players.iter() {
                    let body = NettyRigidBody::new(Some(*velocity), transform.rotation, NettyRigidBodyLocation::Absolute(*location));

                    let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
                        entity,
                        id: player.id(),
                        body,
                        name: player.name().to_owned(),
                        render_distance: Some(*render_distance),
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
                let starting_pos = Vec3::new(0.0, 1900.0, 0.0);
                let location = Location::new(starting_pos, Sector::new(25, 25, 25));
                let velocity = Velocity::default();
                let inventory = generate_player_inventory(player_entity, &items, &mut commands, &needs_data, server_settings.creative);

                let netty_body = NettyRigidBody::new(Some(velocity), Quat::IDENTITY, NettyRigidBodyLocation::Absolute(location));

                let credits = Credits::new(25_000);

                let mut ecmds = commands.entity(player_entity);

                ecmds.insert((
                    location,
                    LockedAxes::ROTATION_LOCKED,
                    RigidBody::Dynamic,
                    velocity,
                    Collider::capsule_y(0.65, 0.25),
                    Friction {
                        coefficient: 0.0,
                        combine_rule: CoefficientCombineRule::Min,
                    },
                    player,
                    ReadMassProperties::default(),
                    inventory,
                    PlayerLooking { rotation: Quat::IDENTITY },
                    LoadingDistance::new(2, 9999),
                    ActiveEvents::COLLISION_EVENTS,
                    Name::new(format!("Player ({name})")),
                    credits,
                ));

                if server_settings.creative {
                    ecmds.insert(Creative);
                }

                lobby.add_player(client_id, player_entity);

                assign_player_world(&player_worlds, player_entity, &location, &mut commands);

                let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
                    entity: player_entity,
                    id: client_id,
                    name,
                    body: netty_body,
                    render_distance: None,
                });

                server.send_message(
                    client_id,
                    NettyChannelServer::Reliable,
                    cosmos_encoder::serialize(&ServerReliableMessages::MOTD {
                        motd: "Welcome to the server!".into(),
                    }),
                );

                server.broadcast_message(NettyChannelServer::Reliable, msg);

                evw_player_join.send(PlayerConnectedEvent { player_entity, client_id });
                evw_sync_registries.send(SyncRegistriesEvent { player_entity });
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
    app.add_event::<PlayerConnectedEvent>();
}
