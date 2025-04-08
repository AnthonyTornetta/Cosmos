//! Server-side automatic component syncing logic

use super::server_entity_syncing::RequestedEntityEvent;
use super::{
    ClientAuthority, ComponentEntityIdentifier, ComponentId, ComponentReplicationMessage, ComponentSyncingSet, RegisterComponentSet,
    ReplicatedComponentData, SyncType, SyncableComponent, SyncedComponentId,
};
use crate::block::data::BlockData;
use crate::entities::player::Player;
use crate::inventory::itemstack::ItemStackData;
use crate::netty::server::ServerLobby;
use crate::netty::sync::{GotComponentToRemoveEvent, GotComponentToSyncEvent};
use crate::netty::system_sets::NetworkingSystemsSet;
use crate::netty::{cosmos_encoder, NettyChannelClient, NettyChannelServer, NoSendEntity};
use crate::persistence::LoadingDistance;
use crate::physics::location::{CosmosBundleSet, Location};
use crate::registry::{identifiable::Identifiable, Registry};
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use bevy::ecs::event::EventReader;
use bevy::ecs::query::Without;
use bevy::ecs::removal_detection::RemovedComponents;
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};
use bevy::ecs::system::Commands;
use bevy::log::warn;
use bevy::prelude::{Parent, With};
use bevy::utils::HashMap;
use bevy::{
    app::{App, Startup, Update},
    ecs::{
        entity::Entity,
        event::EventWriter,
        query::Changed,
        system::{Query, Res, ResMut},
    },
    log::error,
};
use bevy_renet2::renet2::RenetServer;
use renet2::ClientId;

fn server_remove_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToRemoveEvent>,
    mut commands: Commands,
    lobby: Res<ServerLobby>,
    q_piloting: Query<&Pilot>,
) {
    for ev in ev_reader.read() {
        let ComponentId::Custom(id) = ev.component_id else {
            continue;
        };

        let Some(synced_id) = components_registry.try_from_numeric_id(id) else {
            warn!("Missing component with id {}", id);
            continue;
        };

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        let authority = match T::get_sync_type() {
            SyncType::ClientAuthoritative(authority) => authority,
            SyncType::BothAuthoritative(authority) => authority,
            SyncType::ServerAuthoritative => {
                unreachable!("This function cannot be caled if synctype == ServerAuthoritative in the server project.")
            }
        };

        match authority {
            ClientAuthority::Anything => {}
            ClientAuthority::Piloting => {
                let Some(player) = lobby.player_from_id(ev.client_id) else {
                    return;
                };

                let Ok(piloting) = q_piloting.get(player) else {
                    return;
                };

                if piloting.entity != ev.authority_entity {
                    return;
                }
            }
            ClientAuthority::Themselves => {
                let Some(player) = lobby.player_from_id(ev.client_id) else {
                    return;
                };

                if player != ev.authority_entity {
                    return;
                }
            }
        }

        if let Some(mut ecmds) = commands.get_entity(ev.entity) {
            ecmds.remove::<T>();
        }
    }
}

fn server_deserialize_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToSyncEvent>,
    mut commands: Commands,
    lobby: Res<ServerLobby>,
    q_piloting: Query<&Pilot>,
    q_t: Query<&T>,
) {
    for ev in ev_reader.read() {
        let ComponentId::Custom(id) = ev.component_id else {
            continue;
        };

        let Some(synced_id) = components_registry.try_from_numeric_id(id) else {
            warn!("Missing component with id {}", id);
            continue;
        };

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        let authority = match T::get_sync_type() {
            SyncType::ClientAuthoritative(authority) => authority,
            SyncType::BothAuthoritative(authority) => authority,
            SyncType::ServerAuthoritative => {
                unreachable!("This function cannot be caled if synctype == ServerAuthoritative in the server project.")
            }
        };

        match authority {
            ClientAuthority::Anything => {}
            ClientAuthority::Piloting => {
                let Some(player) = lobby.player_from_id(ev.client_id) else {
                    return;
                };

                let Ok(piloting) = q_piloting.get(player) else {
                    return;
                };

                if piloting.entity != ev.authority_entity {
                    return;
                }
            }
            ClientAuthority::Themselves => {
                let Some(player) = lobby.player_from_id(ev.client_id) else {
                    return;
                };

                if player != ev.authority_entity {
                    return;
                }
            }
        }

        if let Some(mut ecmds) = commands.get_entity(ev.entity) {
            let Ok(deserialized) = bincode::deserialize::<T>(&ev.raw_data) else {
                continue;
            };

            if matches!(T::get_sync_type(), SyncType::BothAuthoritative(_)) {
                // Attempt to prevent an endless chain of change detection, causing the client+server to repeatedly sync the same component.
                if q_t.get(ev.entity).map(|x| *x == deserialized).unwrap_or(false) {
                    continue;
                }
            }

            if deserialized.validate() {
                ecmds.try_insert(deserialized);
            }
        }
    }
}

fn recursive_should_load(
    p_loc: &Location,
    entity: Entity,
    q_parent: &Query<(Option<&Location>, Option<&LoadingDistance>, Option<&Parent>)>,
) -> bool {
    let (loc, loading_distance, parent) = q_parent.get(entity).expect("Impossible to fail");

    if let (Some(loc), Some(loading_distance)) = (loc, loading_distance) {
        loading_distance.should_load(p_loc, loc)
    } else if let Some(parent) = parent {
        recursive_should_load(p_loc, parent.get(), q_parent)
    } else {
        false
    }
}

/// Determines if information about this entity should be sent to a player at this location.
pub fn should_be_sent_to(
    p_loc: &Location,
    q_parent: &Query<(Option<&Location>, Option<&LoadingDistance>, Option<&Parent>)>,
    entity_identifier: &ComponentEntityIdentifier,
) -> bool {
    match entity_identifier {
        ComponentEntityIdentifier::Entity(entity) => recursive_should_load(p_loc, *entity, q_parent),
        ComponentEntityIdentifier::StructureSystem { structure_entity, .. } => recursive_should_load(p_loc, *structure_entity, q_parent),
        // TODO: This is probably wrong for dynamic structures like planets
        ComponentEntityIdentifier::BlockData { identifier, .. } => recursive_should_load(p_loc, identifier.block.structure(), q_parent),
        ComponentEntityIdentifier::ItemData { inventory_entity, .. } => recursive_should_load(p_loc, *inventory_entity, q_parent),
    }
}

fn server_send_component<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_parent: Query<(Option<&Location>, Option<&LoadingDistance>, Option<&Parent>)>,
    q_changed_component: Query<
        (Entity, &T, Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>),
        (Without<NoSendEntity>, Changed<T>),
    >,
    q_players: Query<(&Location, &Player)>,
    mut server: ResMut<RenetServer>,
) {
    if q_changed_component.is_empty() {
        return;
    }

    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

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
                raw_data: bincode::serialize(component).expect("Failed to serialize component!"),
            })
            .collect::<Vec<ReplicatedComponentData>>();

        server.send_message(
            player.client_id(),
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::Custom(id.id()),
                replicated: replicated_data,
            }),
        );
    });
}

fn server_sync_removed_components<T: SyncableComponent>(
    mut removed_components: RemovedComponents<T>,
    q_entity_identifier: Query<(Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>)>,
    id_registry: Res<Registry<SyncedComponentId>>,
    mut server: ResMut<RenetServer>,
) {
    if removed_components.is_empty() {
        return;
    }

    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

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
                component_id: ComponentId::Custom(id.id()),
                entity_identifier,
            }),
        );
    }
}

fn on_request_component<T: SyncableComponent>(
    q_t: Query<(&T, Option<&StructureSystem>, Option<&ItemStackData>, Option<&BlockData>), Without<NoSendEntity>>,
    q_parent: Query<(Option<&Location>, Option<&LoadingDistance>, Option<&Parent>)>,
    mut ev_reader: EventReader<RequestedEntityEvent>,
    id_registry: Res<Registry<SyncedComponentId>>,
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

        let Ok((component, structure_system, is_data, block_data)) = q_t.get(ev.entity) else {
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
            raw_data: bincode::serialize(component).expect("Failed to serialize component."),
            entity_identifier,
        });
    }

    for (client_id, replicated_component) in comps_to_send {
        let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
            error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
            return;
        };

        server.send_message(
            client_id,
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::Custom(id.id()),
                replicated: replicated_component,
            }),
        );
    }
}

fn server_receive_components(
    mut server: ResMut<RenetServer>,
    mut ev_writer_sync: EventWriter<GotComponentToSyncEvent>,
    mut ev_writer_remove: EventWriter<GotComponentToRemoveEvent>,
    q_structure_systems: Query<&StructureSystems>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::ComponentReplication) {
            let Ok(msg) = cosmos_encoder::deserialize::<ComponentReplicationMessage>(&message) else {
                warn!("Bad deserialization");
                continue;
            };

            match msg {
                ComponentReplicationMessage::ComponentReplication { component_id, replicated } => {
                    for ReplicatedComponentData {
                        entity_identifier,
                        raw_data,
                    } in replicated
                    {
                        let (entity, authority_entity) = match entity_identifier {
                            ComponentEntityIdentifier::Entity(entity) => (entity, entity),
                            ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
                                let Ok(structure_systems) = q_structure_systems.get(structure_entity) else {
                                    warn!("Bad structure entity {structure_entity:?}");
                                    continue;
                                };

                                let Some(system_entity) = structure_systems.get_system_entity(id) else {
                                    warn!("Bad structure system id {id:?}");
                                    continue;
                                };

                                (system_entity, structure_entity)
                            }
                            ComponentEntityIdentifier::ItemData {
                                inventory_entity: _,
                                item_slot: _,
                                server_data_entity: _,
                            } => {
                                warn!("Client-authoritiative syncing of itemdata not yet implemented");

                                continue;
                            }
                            ComponentEntityIdentifier::BlockData {
                                identifier: _,
                                server_data_entity: _,
                            } => {
                                warn!("Client-authoritiative syncing of blockdata not yet implemented");

                                continue;
                            }
                        };

                        ev_writer_sync.send(GotComponentToSyncEvent {
                            client_id,
                            component_id,
                            entity,
                            authority_entity,
                            raw_data,
                        });
                    }
                }
                ComponentReplicationMessage::RemovedComponent {
                    component_id,
                    entity_identifier,
                } => {
                    let (entity, authority_entity) = match entity_identifier {
                        ComponentEntityIdentifier::Entity(entity) => (entity, entity),
                        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
                            let Ok(structure_systems) = q_structure_systems.get(structure_entity) else {
                                warn!("Bad structure entity {structure_entity:?}");
                                continue;
                            };

                            let Some(system_entity) = structure_systems.get_system_entity(id) else {
                                warn!("Bad structure system id {id:?}");
                                continue;
                            };

                            (system_entity, structure_entity)
                        }
                        ComponentEntityIdentifier::ItemData {
                            inventory_entity: _,
                            item_slot: _,
                            server_data_entity: _,
                        } => {
                            warn!("Client-authoritiative syncing of itemdata not yet implemented");

                            continue;
                        }
                        ComponentEntityIdentifier::BlockData {
                            identifier: _,
                            server_data_entity: _,
                        } => {
                            warn!("Client-authoritiative syncing of blockdata not yet implemented");

                            continue;
                        }
                    };

                    ev_writer_remove.send(GotComponentToRemoveEvent {
                        client_id,
                        component_id,
                        entity,
                        authority_entity,
                    });
                }
            }
        }
    }
}

pub(super) fn setup_server(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            .after(CosmosBundleSet::HandleCosmosBundles)
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    );

    app.configure_sets(
        Update,
        ComponentSyncingSet::ReceiveComponents.in_set(NetworkingSystemsSet::ReceiveMessages),
    );

    app.add_systems(Update, server_receive_components.in_set(ComponentSyncingSet::PreComponentSyncing))
        .add_event::<RequestedEntityEvent>();
}

fn register_component<T: SyncableComponent>(mut registry: ResMut<Registry<SyncedComponentId>>) {
    registry.register(SyncedComponentId {
        unlocalized_name: T::get_component_unlocalized_name().to_owned(),
        id: 0,
    });
}

pub(super) fn sync_component_server<T: SyncableComponent>(app: &mut App) {
    app.add_systems(
        Startup,
        register_component::<T>
            .in_set(RegisterComponentSet::RegisterComponent)
            .ambiguous_with(RegisterComponentSet::RegisterComponent),
    );

    match T::get_sync_type() {
        SyncType::ServerAuthoritative => {
            app.add_systems(
                Update,
                (
                    on_request_component::<T>,
                    server_send_component::<T>,
                    server_sync_removed_components::<T>,
                )
                    .chain()
                    .run_if(resource_exists::<RenetServer>)
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
        }
        SyncType::ClientAuthoritative(_) => {
            app.add_systems(
                Update,
                (server_deserialize_component::<T>, server_remove_component::<T>)
                    .chain()
                    .in_set(ComponentSyncingSet::ReceiveComponents),
            );
        }
        SyncType::BothAuthoritative(_) => {
            app.add_systems(
                Update,
                (
                    on_request_component::<T>,
                    server_send_component::<T>,
                    server_sync_removed_components::<T>,
                )
                    .chain()
                    .run_if(resource_exists::<RenetServer>)
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
            app.add_systems(
                Update,
                (server_deserialize_component::<T>, server_remove_component::<T>)
                    .chain()
                    .in_set(ComponentSyncingSet::ReceiveComponents),
            );
        }
    }
}
