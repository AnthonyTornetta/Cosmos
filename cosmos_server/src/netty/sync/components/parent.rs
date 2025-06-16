use bevy::{platform::collections::HashMap, prelude::*};
use cosmos_core::{
    block::data::BlockData,
    entities::player::Player,
    inventory::itemstack::ItemStackData,
    netty::{
        NettyChannelServer, NoSendEntity, cosmos_encoder,
        sync::{
            ComponentEntityIdentifier, ComponentId, ComponentReplicationMessage, ComponentSyncingSet, ReplicatedComponentData,
            server_entity_syncing::RequestedEntityEvent, server_syncing::SyncTo,
        },
    },
    prelude::StructureSystem,
};
use renet::{ClientId, RenetServer};

fn on_request_parent(
    q_component: Query<(&ChildOf, Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>), Without<NoSendEntity>>,
    mut ev_reader: EventReader<RequestedEntityEvent>,
    mut server: ResMut<RenetServer>,
) {
    let mut comps_to_send: HashMap<ClientId, Vec<ReplicatedComponentData>> = HashMap::new();

    for ev in ev_reader.read() {
        let Ok((component, structure_system, is_data, block_data)) = q_component.get(ev.entity) else {
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
                server_data_entity: ev.entity,
            }
        } else if let Some(block_data) = block_data {
            ComponentEntityIdentifier::BlockData {
                identifier: block_data.identifier,
                server_data_entity: ev.entity,
            }
        } else {
            info!("Requested parent for {:?}", ev.entity);
            ComponentEntityIdentifier::Entity(ev.entity)
        };

        comps_to_send.entry(ev.client_id).or_default().push(ReplicatedComponentData {
            raw_data: cosmos_encoder::serialize_uncompressed(&component.parent()),
            entity_identifier,
        });
    }

    for (client_id, replicated_component) in comps_to_send {
        server.send_message(
            client_id,
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::ChildOf,
                replicated: replicated_component,
            }),
        );
    }
}

fn on_change_parent(
    q_changed_component: Query<
        (
            Entity,
            &ChildOf,
            &SyncTo,
            Option<&StructureSystem>,
            Option<&ItemStackData>,
            Option<&BlockData>,
        ),
        (Without<NoSendEntity>, Changed<ChildOf>),
    >,
    q_players: Query<&Player>,
    mut server: ResMut<RenetServer>,
) {
    if q_changed_component.is_empty() {
        return;
    }

    q_players.iter().for_each(|player| {
        let replicated_data = q_changed_component
            .iter()
            .flat_map(|(entity, component, sync_to, structure_system, is_data, block_data)| {
                if !sync_to.should_sync_to(player.client_id()) {
                    return None;
                }

                let entity_identifier = if let Some(structure_system) = structure_system {
                    ComponentEntityIdentifier::StructureSystem {
                        structure_entity: structure_system.structure_entity(),
                        id: structure_system.id(),
                    }
                } else if let Some(is_data) = is_data {
                    ComponentEntityIdentifier::ItemData {
                        inventory_entity: is_data.inventory_pointer.0,
                        item_slot: is_data.inventory_pointer.1,
                        server_data_entity: entity,
                    }
                } else if let Some(block_data) = block_data {
                    ComponentEntityIdentifier::BlockData {
                        identifier: block_data.identifier,
                        server_data_entity: entity,
                    }
                } else {
                    ComponentEntityIdentifier::Entity(entity)
                };

                Some((component, entity_identifier))
            })
            .map(|(component, identifier)| ReplicatedComponentData {
                entity_identifier: identifier,
                raw_data: cosmos_encoder::serialize_uncompressed(&component.parent()),
            })
            .collect::<Vec<ReplicatedComponentData>>();

        server.send_message(
            player.client_id(),
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::ChildOf,
                replicated: replicated_data,
            }),
        );
    });
}

fn on_remove_parent(
    mut removed_components: RemovedComponents<ChildOf>,
    q_entity_identifier: Query<(Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>)>,
    mut server: ResMut<RenetServer>,
) {
    if removed_components.is_empty() {
        return;
    }

    for removed_ent in removed_components.read() {
        // Ignores despawned entities
        let Ok((structure_system, is_data, block_data)) = q_entity_identifier.get(removed_ent) else {
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
                server_data_entity: removed_ent,
            }
        } else if let Some(block_data) = block_data {
            ComponentEntityIdentifier::BlockData {
                identifier: block_data.identifier,
                server_data_entity: removed_ent,
            }
        } else {
            ComponentEntityIdentifier::Entity(removed_ent)
        };

        server.broadcast_message(
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::RemovedComponent {
                component_id: ComponentId::ChildOf,
                entity_identifier,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_request_parent, on_change_parent, on_remove_parent)
            .chain()
            .run_if(resource_exists::<RenetServer>)
            .in_set(ComponentSyncingSet::DoComponentSyncing),
    );
}
