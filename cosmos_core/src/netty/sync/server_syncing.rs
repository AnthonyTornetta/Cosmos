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
use crate::netty::{NettyChannelClient, NettyChannelServer, NoSendEntity, cosmos_encoder};
use crate::physics::location::LocationPhysicsSet;
use crate::registry::{Registry, identifiable::Identifiable};
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use crate::utils::ecs::{FixedUpdateRemovedComponents, register_fixed_update_removed_component};
use bevy::app::FixedUpdate;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::Without;
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::system::Commands;
use bevy::log::{info, warn};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::{Component, IntoScheduleConfigs, With};
use bevy::reflect::Reflect;
use bevy::{
    app::{App, Startup},
    ecs::{
        entity::Entity,
        event::EventWriter,
        query::Changed,
        system::{Query, Res, ResMut},
    },
    log::error,
};
use bevy_renet::renet::RenetServer;
use renet::ClientId;

#[derive(Component, Debug, Reflect)]
/// This is a flag placed onto players that are ready to receive components from the server.
///
/// This mean the player has already triggered a [`ClientFinishedReceivingRegistriesEvent`].
pub struct ReadyForSyncing;

#[derive(Component, Debug, Reflect, Clone, Default)]
/// Contains the list of clients this entity should be synced to
pub struct SyncTo(HashSet<ClientId>);

impl SyncTo {
    /// Creates a new list of clients that should be synced with
    pub fn new(clients: HashSet<ClientId>) -> Self {
        Self(clients)
    }

    /// Iterates over all clients this should sync with
    pub fn iter(&self) -> impl Iterator<Item = &ClientId> {
        self.0.iter()
    }

    /// Returns if this should be synced to this client id.
    pub fn should_sync_to(&self, client_id: ClientId) -> bool {
        self.0.contains(&client_id)
    }
}

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

        if let Ok(mut ecmds) = commands.get_entity(ev.entity) {
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

        if let Ok(mut ecmds) = commands.get_entity(ev.entity) {
            let Ok(deserialized) = cosmos_encoder::deserialize_uncompressed::<T>(&ev.raw_data) else {
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

fn server_send_component<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_changed_component: Query<
        (
            Entity,
            &T,
            &SyncTo,
            Option<&StructureSystem>,
            Option<&ItemStackData>,
            Option<&BlockData>,
        ),
        (Without<NoSendEntity>, Changed<T>),
    >,
    q_players: Query<&Player, With<ReadyForSyncing>>,
    mut server: ResMut<RenetServer>,
) {
    if q_changed_component.is_empty() {
        return;
    }

    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

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

                if T::debug() {
                    info!(
                        "Syncing change to {} on entity {entity_identifier:?}",
                        T::get_component_unlocalized_name()
                    );
                }

                Some((component, entity_identifier))
            })
            .map(|(component, identifier)| ReplicatedComponentData {
                entity_identifier: identifier,
                raw_data: cosmos_encoder::serialize_uncompressed(component),
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
    removed_components: FixedUpdateRemovedComponents<T>,
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

        if T::debug() {
            info!(
                "Syncing removed {} on entity {entity_identifier:?}",
                T::get_component_unlocalized_name()
            );
        }

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
    mut ev_reader: EventReader<RequestedEntityEvent>,
    id_registry: Res<Registry<SyncedComponentId>>,
    mut server: ResMut<RenetServer>,
) {
    let mut comps_to_send: HashMap<ClientId, Vec<ReplicatedComponentData>> = HashMap::new();

    for ev in ev_reader.read() {
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

        if T::debug() {
            info!("Requested {} on entity {entity_identifier:?}", T::get_component_unlocalized_name());
        }

        comps_to_send.entry(ev.client_id).or_default().push(ReplicatedComponentData {
            raw_data: cosmos_encoder::serialize_uncompressed(component),
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

                        ev_writer_sync.write(GotComponentToSyncEvent {
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

                    ev_writer_remove.write(GotComponentToRemoveEvent {
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
        FixedUpdate,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            // Should this be before?
            .after(LocationPhysicsSet::DoPhysics)
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    );

    app.configure_sets(
        FixedUpdate,
        ComponentSyncingSet::ReceiveComponents.in_set(NetworkingSystemsSet::ReceiveMessages),
    );

    app.add_systems(
        FixedUpdate,
        server_receive_components.in_set(ComponentSyncingSet::PreComponentSyncing),
    )
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

    if T::get_sync_type().is_server_authoritative() {
        register_fixed_update_removed_component::<T>(app);
        app.add_systems(
            FixedUpdate,
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
    if T::get_sync_type().is_client_authoritative() {
        app.add_systems(
            FixedUpdate,
            (server_deserialize_component::<T>, server_remove_component::<T>)
                .chain()
                .in_set(ComponentSyncingSet::ReceiveComponents),
        );
    }
}
