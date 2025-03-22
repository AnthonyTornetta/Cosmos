use bevy::{prelude::*, utils::HashMap};
use cosmos_core::{
    block::data::BlockData,
    entities::player::Player,
    inventory::itemstack::ItemStackData,
    netty::{
        cosmos_encoder,
        server::ServerLobby,
        sync::{
            server_entity_syncing::RequestedEntityEvent, server_syncing::should_be_sent_to, ComponentEntityIdentifier, ComponentId,
            ComponentReplicationMessage, ComponentSyncingSet, ReplicatedComponentData,
        },
        NettyChannelServer, NoSendEntity,
    },
    persistence::LoadingDistance,
    physics::location::Location,
    prelude::StructureSystem,
};
use renet2::{ClientId, RenetServer};

fn on_request_parent(
    q_component: Query<(&Parent, Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>), Without<NoSendEntity>>,
    q_parent: Query<(Option<&Location>, Option<&LoadingDistance>, Option<&Parent>)>,
    mut ev_reader: EventReader<RequestedEntityEvent>,
    mut server: ResMut<RenetServer>,
    q_players: Query<&Location, With<Player>>,
    lobby: Res<ServerLobby>,
) {
    let mut comps_to_send: HashMap<ClientId, Vec<ReplicatedComponentData>> = HashMap::new();

    for ev in ev_reader.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            continue;
        };
        let Ok(p_loc) = q_players.get(player_ent) else {
            continue;
        };

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
            ComponentEntityIdentifier::Entity(ev.entity)
        };

        if !should_be_sent_to(p_loc, &q_parent, &entity_identifier) {
            continue;
        }

        comps_to_send.entry(ev.client_id).or_default().push(ReplicatedComponentData {
            raw_data: bincode::serialize(&component.get()).expect("Failed to serialize component."),
            entity_identifier,
        });
    }

    for (client_id, replicated_component) in comps_to_send {
        server.send_message(
            client_id,
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::Parent,
                replicated: replicated_component,
            }),
        );
    }
}

fn on_change_parent(
    q_parent: Query<(Option<&Location>, Option<&LoadingDistance>, Option<&Parent>)>,
    q_changed_component: Query<
        (
            Entity,
            &Parent,
            Option<&StructureSystem>,
            Option<&ItemStackData>,
            Option<&BlockData>,
        ),
        (Without<NoSendEntity>, Changed<Parent>),
    >,
    q_players: Query<(&Location, &Player)>,
    mut server: ResMut<RenetServer>,
) {
    if q_changed_component.is_empty() {
        return;
    }

    q_players.iter().for_each(|(p_loc, player)| {
        let replicated_data = q_changed_component
            .iter()
            .map(|(entity, component, structure_system, is_data, block_data)| {
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

                (component, entity_identifier)
            })
            .filter(|(_, entity_identifier)| should_be_sent_to(p_loc, &q_parent, entity_identifier))
            .map(|(component, identifier)| ReplicatedComponentData {
                entity_identifier: identifier,
                raw_data: bincode::serialize(&component.get()).expect("Failed to serialize component!"),
            })
            .collect::<Vec<ReplicatedComponentData>>();

        server.send_message(
            player.client_id(),
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::Parent,
                replicated: replicated_data,
            }),
        );
    });
}

fn on_remove_parent(
    mut removed_components: RemovedComponents<Parent>,
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
                component_id: ComponentId::Parent,
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
