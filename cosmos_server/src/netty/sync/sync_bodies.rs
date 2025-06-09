//! Handles the syncing of entity's rigidbodies + velocities

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::data::BlockData,
    ecs::{NeedsDespawned, despawn_needed},
    entities::player::{Player, render_distance::RenderDistance},
    inventory::itemstack::ItemStackData,
    netty::{
        NettyChannelServer, NoSendEntity, cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server_reliable_messages::ServerReliableMessages,
        server_unreliable_messages::ServerUnreliableMessages,
        sync::{ComponentEntityIdentifier, server_entity_syncing::RequestedEntityEvent, server_syncing::ReadyForSyncing},
        system_sets::NetworkingSystemsSet,
    },
    physics::location::{Location, LocationPhysicsSet},
    structure::systems::StructureSystem,
};

use crate::netty::network_helpers::NetworkTick;

use super::flags::SyncTo;

#[derive(Component)]
/// Does not send a despawn message to the client when this entity is despawned.
///
/// This only works if the entity is despawned via the `NeedsDespawned` component.
pub struct DontNotifyClientOfDespawn;

/// Sends bodies to players only if it's within their render distance.
fn send_bodies(
    players: &Query<(&Player, &RenderDistance), With<ReadyForSyncing>>,
    bodies: &[(Entity, NettyRigidBody, &SyncTo)],
    server: &mut RenetServer,
    tick: &NetworkTick,
) {
    for (player, _) in players.iter() {
        let players_bodies: Vec<(Entity, NettyRigidBody)> = bodies
            .iter()
            .filter(|(_, _, sync_to)| sync_to.should_sync_to(player.client_id()))
            .map(|(ent, net_rb, _)| (*ent, *net_rb))
            .collect();

        if !players_bodies.is_empty() {
            let sync_message = ServerUnreliableMessages::BulkBodies {
                time_stamp: tick.0,
                bodies: players_bodies,
            };

            let message = cosmos_encoder::serialize(&sync_message);
            server.send_message(player.client_id(), NettyChannelServer::Unreliable, message.clone());
        }
    }
}

fn server_sync_bodies(
    mut server: ResMut<RenetServer>,
    mut tick: ResMut<NetworkTick>,
    entities: Query<
        (Entity, &Transform, &Location, Option<&Velocity>, &SyncTo, Option<&Parent>),
        (Or<(Changed<Location>, Changed<Transform>, Changed<SyncTo>)>, Without<NoSendEntity>),
    >,
    players: Query<(&Player, &RenderDistance), With<ReadyForSyncing>>,
    // Often children will not have locations or loading distances, but still need to by synced
    // q_children_need_synced: Query<
    //     (Entity, Option<&Velocity>, &Transform, &Parent),
    //     (Without<LoadingDistance>, Without<NoSendEntity>, Without<Location>),
    // >,
    // q_loading_distance: Query<(&Location, &LoadingDistance)>,
    // q_parent: Query<&Parent>,
) {
    tick.0 += 1;

    let mut bodies = Vec::with_capacity(20);

    for (entity, transform, location, velocity, sync_to, parent) in entities.iter() {
        bodies.push((
            entity,
            NettyRigidBody::new(
                velocity.copied(),
                transform.rotation,
                match parent.map(|p| p.get()) {
                    Some(parent_entity) => NettyRigidBodyLocation::Relative(transform.translation, parent_entity),
                    None => NettyRigidBodyLocation::Absolute(*location),
                },
            ),
            sync_to,
        ));

        // The packet size can only be so big, so limit syncing to 20 per packet
        if bodies.len() >= 20 {
            send_bodies(&players, &bodies, &mut server, &tick);
            bodies.clear();
        }
    }

    // for (ent, velocity, transform, parent) in q_children_need_synced.iter() {
    //     let mut info = None;

    //     let mut cur_ent = parent.get();
    //     while info.is_none() {
    //         if let Ok((loc, load_dist)) = q_loading_distance.get(cur_ent) {
    //             info = Some((*loc, *load_dist));
    //         } else {
    //             if let Ok(next_ent) = q_parent.get(cur_ent) {
    //                 cur_ent = next_ent.get();
    //             } else {
    //                 break;
    //             }
    //         }
    //     }

    //     let Some((parent_loc, parent_loading_distance)) = info else {
    //         continue;
    //     };

    // }

    if !bodies.is_empty() {
        send_bodies(&players, &bodies, &mut server, &tick);
    }
}

fn notify_client_of_successful_entity_request(
    mut server: ResMut<RenetServer>,
    mut event_reader: EventReader<RequestedEntityEvent>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        if commands.get_entity(ev.entity).is_some() {
            server.send_message(
                ev.client_id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::RequestedEntityReceived(ev.entity)),
            );
        }
    }
}

fn notify_despawned_entities(
    removed_components: Query<Entity, (With<NeedsDespawned>, (Without<DontNotifyClientOfDespawn>, Without<NoSendEntity>))>,
    q_identifier: Query<(Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>)>,
    mut server: ResMut<RenetServer>,
) {
    for killed_entity in removed_components.iter() {
        let Ok((structure_system, is_data, block_data)) = q_identifier.get(killed_entity) else {
            continue;
        };

        let entity_identifier = if let Some(structure_system) = structure_system {
            ComponentEntityIdentifier::StructureSystem {
                structure_entity: structure_system.structure_entity(),
                id: structure_system.id(),
            }
        } else if let Some(is_data) = is_data {
            ComponentEntityIdentifier::ItemData {
                inventory_entity: is_data.inventory_pointer.0,
                item_slot: is_data.inventory_pointer.1,
                server_data_entity: killed_entity,
            }
        } else if let Some(block_data) = block_data {
            ComponentEntityIdentifier::BlockData {
                identifier: block_data.identifier,
                server_data_entity: killed_entity,
            }
        } else {
            ComponentEntityIdentifier::Entity(killed_entity)
        };

        info!("Notifying of entity despawn -- {entity_identifier:?}");

        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::EntityDespawn { entity: entity_identifier }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            notify_client_of_successful_entity_request,
            server_sync_bodies.after(LocationPhysicsSet::DoPhysics),
        )
            .in_set(NetworkingSystemsSet::SyncComponents),
    )
    .add_systems(First, notify_despawned_entities.before(despawn_needed));
}
