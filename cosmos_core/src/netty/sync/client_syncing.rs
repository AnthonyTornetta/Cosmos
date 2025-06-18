//! Exposes [`ClientReceiveComponents::ClientReceiveComponents`] - this is to remove ambiguity

use std::marker::PhantomData;

use super::mapping::{NetworkMapping, ServerEntity};
use super::{
    ClientAuthority, ComponentEntityIdentifier, ComponentId, ComponentReplicationMessage, ComponentSyncingSet, GotComponentToRemoveEvent,
    ReplicatedComponentData, SyncType, SyncableComponent, SyncedComponentId,
};
use crate::block::data::BlockData;
use crate::ecs::{NeedsDespawned, add_multi_statebound_resource};
use crate::events::block_events::BlockDataChangedEvent;
use crate::inventory::Inventory;
use crate::inventory::itemstack::ItemStackData;
use crate::netty::client::LocalPlayer;
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::sync::mapping::Mappable;
use crate::netty::system_sets::NetworkingSystemsSet;
use crate::netty::{NettyChannelClient, cosmos_encoder};
use crate::netty::{NettyChannelServer, NoSendEntity};
use crate::registry::{Registry, identifiable::Identifiable};
use crate::state::GameState;
use crate::structure::Structure;
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use crate::utils::ecs::{FixedUpdateRemovedComponents, register_fixed_update_removed_component};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;
use bevy::time::Time;
use bevy::{
    app::App,
    ecs::{
        entity::Entity,
        event::EventWriter,
        query::Changed,
        system::{Query, Res, ResMut},
    },
    log::error,
};
use bevy_renet::renet::RenetClient;

#[derive(Resource)]
struct StoredComponents<T: SyncableComponent>(HashMap<Entity, (Vec<u8>, f32)>, PhantomData<T>);

impl<T: SyncableComponent> Default for StoredComponents<T> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

fn client_add_stored_components<T: SyncableComponent>(
    time: Res<Time>,
    mut commands: Commands,
    mut sc: ResMut<StoredComponents<T>>,
    mapping: Res<NetworkMapping>,
    q_t: Query<&T>,
) {
    let hm = &mut sc.as_mut().0;
    let ents = hm.keys().copied().collect::<Vec<Entity>>();
    for ent in ents {
        if let Ok(mut ecmds) = commands.get_entity(ent) {
            let (c, _) = hm.remove(&ent).expect("Must exist");

            let mut component =
                cosmos_encoder::deserialize_uncompressed::<T>(&c).expect("Failed to deserialize component sent from server!");

            let Some(mapped) = component.convert_entities_server_to_client(&mapping) else {
                warn!("Couldn't convert entities for {}!", T::get_component_unlocalized_name());
                continue;
            };

            component = mapped;

            if matches!(T::get_sync_type(), SyncType::BothAuthoritative(_)) {
                // Attempt to prevent an endless chain of change detection, causing the client+server to repeatedly sync the same component.
                if q_t.get(ent).map(|x| *x == component).unwrap_or(false) {
                    continue;
                }
            }

            if component.validate() {
                if T::debug() {
                    info!(
                        "Attempting to insert {:?} into entity {ent:?} now that the entity exists {component:?}",
                        T::get_component_unlocalized_name()
                    );
                }
                ecmds.try_insert(component);
                if T::debug() {
                    ecmds.log_components();
                }
            }
        } else {
            let (_, t) = hm.get_mut(&ent).expect("Must exist");
            *t += time.delta_secs();

            if *t > 5.0 {
                hm.remove(&ent);
                warn!("No entity cmds for synced entity component - (entity {ent:?})");
            }
        }
    }
}

fn client_deserialize_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToSyncEvent>,
    mut commands: Commands,
    mapping: Res<NetworkMapping>,
    q_t: Query<&T>,
    mut stored_components: ResMut<StoredComponents<T>>,
) {
    for ev in ev_reader.read() {
        let ComponentId::Custom(id) = ev.component_id else {
            continue;
        };

        let synced_id = components_registry
            .try_from_numeric_id(id)
            .unwrap_or_else(|| panic!("Missing component with id {id}\n\n{components_registry:?}\n\n"));

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        if let Ok(mut ecmds) = commands.get_entity(ev.entity) {
            let mut component =
                cosmos_encoder::deserialize_uncompressed::<T>(&ev.raw_data).expect("Failed to deserialize component sent from server!");

            let Some(mapped) = component.convert_entities_server_to_client(&mapping) else {
                warn!("Couldn't convert entities for {}!", T::get_component_unlocalized_name());
                continue;
            };

            component = mapped;

            if matches!(T::get_sync_type(), SyncType::BothAuthoritative(_)) {
                // Attempt to prevent an endless chain of change detection, causing the client+server to repeatedly sync the same component.
                if q_t.get(ev.entity).map(|x| *x == component).unwrap_or(false) {
                    continue;
                }
            }

            if component.validate() {
                if T::debug() {
                    info!(
                        "Attempting to insert {:?} into entity {:?}",
                        T::get_component_unlocalized_name(),
                        ev.entity
                    );
                }
                ecmds.try_insert(component);
                if T::debug() {
                    ecmds.log_components();
                }
            }
        } else {
            if T::debug() {
                info!(
                    "Going to try to insert {:?} into entity {:?} later (entity doesn't exist yet)",
                    T::get_component_unlocalized_name(),
                    ev.entity
                );
            }

            // Try again later
            stored_components.0.insert(ev.entity, (ev.raw_data.clone(), 0.0));
        }
    }
}

fn client_remove_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToRemoveEvent>,
    mut commands: Commands,
) {
    for ev in ev_reader.read() {
        let ComponentId::Custom(id) = ev.component_id else {
            continue;
        };

        let synced_id = components_registry
            .try_from_numeric_id(id)
            .unwrap_or_else(|| panic!("Missing component with id {id}"));

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        if let Ok(mut ecmds) = commands.get_entity(ev.entity) {
            if T::debug() {
                info!(
                    "Removing component {} from entity {:?}",
                    T::get_component_unlocalized_name(),
                    ev.entity
                );
            }
            ecmds.remove::<T>();
        }
    }
}

fn client_send_components<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_changed_component: Query<(Entity, &T, Option<&StructureSystem>, Option<&ItemStackData>), (Without<NoSendEntity>, Changed<T>)>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_local_piloting: Query<&Pilot, With<LocalPlayer>>,
) {
    let authority = match T::get_sync_type() {
        SyncType::ClientAuthoritative(authority) => authority,
        SyncType::BothAuthoritative(authority) => authority,
        SyncType::ServerAuthoritative => {
            unreachable!("This function cannot be caled if synctype == ServerAuthoritative in the server project.")
        }
    };

    if q_changed_component.is_empty() {
        return;
    }
    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

    let data_to_sync = q_changed_component
        .iter()
        .flat_map(|(entity, component, structure_system, is_data)| {
            if !component.should_send_to_server(&mapping) {
                return None;
            }

            let entity_identifier = compute_entity_identifier(structure_system, &mapping, is_data, entity);

            let Some((entity_identifier, authority_entity)) = entity_identifier else {
                warn!("Invalid entity found - {entity_identifier:?} - no match on server entities!");
                return None;
            };

            // The server checks this anyway, but save the client+server some bandwidth
            match authority {
                ClientAuthority::Anything => {}
                ClientAuthority::Piloting => {
                    let Ok(piloting) = q_local_piloting.single() else {
                        return None;
                    };

                    if piloting.entity != authority_entity {
                        return None;
                    }
                }
                ClientAuthority::Themselves => {
                    if !q_local_player.contains(entity) {
                        return None;
                    }
                }
            }

            let raw_data = if T::needs_entity_conversion() {
                let Some(mapped) = component.clone().convert_entities_client_to_server(&mapping) else {
                    error!(
                        "Failed to map component {} on entity {entity:?} to server version!",
                        T::get_component_unlocalized_name()
                    );
                    return None;
                };

                cosmos_encoder::serialize_uncompressed(&mapped)
            } else {
                cosmos_encoder::serialize_uncompressed(component)
            };

            Some(ReplicatedComponentData {
                raw_data,
                entity_identifier,
            })
        })
        .collect::<Vec<ReplicatedComponentData>>();

    if !data_to_sync.is_empty() {
        client.send_message(
            NettyChannelClient::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: ComponentId::Custom(id.id()),
                replicated: data_to_sync,
            }),
        );
    }
}

fn client_send_removed_components<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    removed_components: FixedUpdateRemovedComponents<T>,
    q_entity_identifier: Query<(Option<&StructureSystem>, Option<&ItemStackData>)>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_local_piloting: Query<&Pilot, With<LocalPlayer>>,
) {
    let authority = match T::get_sync_type() {
        SyncType::ClientAuthoritative(authority) => authority,
        SyncType::BothAuthoritative(authority) => authority,
        SyncType::ServerAuthoritative => {
            unreachable!("This function cannot be caled if synctype == ServerAuthoritative in the server project.")
        }
    };

    if removed_components.is_empty() {
        return;
    }
    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

    for removed_ent in removed_components.read() {
        let Ok((structure_system, is_data)) = q_entity_identifier.get(removed_ent) else {
            continue;
        };

        let entity_identifier = compute_entity_identifier(structure_system, &mapping, is_data, removed_ent);

        let Some((entity_identifier, authority_entity)) = entity_identifier else {
            warn!("Invalid entity found - {entity_identifier:?} - no match on server entities!");
            return;
        };

        // The server checks this anyway, but save the client+server some bandwidth
        match authority {
            ClientAuthority::Anything => {}
            ClientAuthority::Piloting => {
                let Ok(piloting) = q_local_piloting.single() else {
                    return;
                };

                if piloting.entity != authority_entity {
                    return;
                }
            }
            ClientAuthority::Themselves => {
                if !q_local_player.contains(removed_ent) {
                    return;
                }
            }
        }

        client.send_message(
            NettyChannelClient::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::RemovedComponent {
                component_id: ComponentId::Custom(id.id()),
                entity_identifier,
            }),
        )
    }
}

fn compute_entity_identifier(
    structure_system: Option<&StructureSystem>,
    mapping: &NetworkMapping,
    is_data: Option<&ItemStackData>,
    entity: Entity,
) -> Option<(ComponentEntityIdentifier, Entity)> {
    if let Some(structure_system) = structure_system {
        mapping.server_from_client(&structure_system.structure_entity()).map(|e| {
            (
                ComponentEntityIdentifier::StructureSystem {
                    structure_entity: e,
                    id: structure_system.id(),
                },
                structure_system.structure_entity(),
            )
        })
    } else if let Some(is_data) = is_data {
        mapping.server_from_client(&is_data.inventory_pointer.0).map(|inv_e| {
            (
                ComponentEntityIdentifier::ItemData {
                    inventory_entity: inv_e,
                    item_slot: is_data.inventory_pointer.1,
                    // Server does not need this?
                    server_data_entity: entity,
                },
                is_data.inventory_pointer.0,
            )
        })
    } else {
        mapping
            .server_from_client(&entity)
            .map(|e| (ComponentEntityIdentifier::Entity(e), entity))
    }
}

#[derive(Resource, Default)]
struct WaitingData(Vec<(ComponentId, ReplicatedComponentData)>);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Receives auto-synced components from the server
pub enum ClientReceiveComponents {
    /// Receives auto-synced components from the server
    ClientReceiveComponents,
}

fn client_receive_components(
    mut client: ResMut<RenetClient>,
    mut ev_writer_sync: EventWriter<GotComponentToSyncEvent>,
    mut ev_writer_remove: EventWriter<GotComponentToRemoveEvent>,
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
    q_structure_systems: Query<&StructureSystems>,
    mut q_inventory: Query<&mut Inventory>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut commands: Commands,
    mut waiting_data: ResMut<WaitingData>,
    mut q_structure: Query<&mut Structure>,
    q_block_data: Query<&BlockData>,
) {
    waiting_data.0.retain(|(component_id, c)| {
        repl_comp_data(
            &q_block_data,
            &mut network_mapping,
            &q_structure_systems,
            &mut q_inventory,
            &mut commands,
            &mut q_structure,
            &mut ev_writer_sync,
            &mut evw_block_data_changed,
            *component_id,
            c.clone(),
        )
        .is_some()
    });

    let mut done = HashSet::new();
    waiting_data.0.retain(|(_, item)| {
        let ComponentEntityIdentifier::Entity(e) = item.entity_identifier else {
            return true;
        };

        if !done.insert(e) {
            return false;
        }

        let ent = commands.spawn(Name::new("Loading auto synced component from server")).id();

        network_mapping.add_mapping(ent, e);

        false
    });

    while let Some(message) = client.receive_message(NettyChannelServer::ComponentReplication) {
        let msg: ComponentReplicationMessage = cosmos_encoder::deserialize(&message).unwrap_or_else(|e| {
            panic!("Failed to parse component replication message from server! Bytes:\n{message:?}\nError: {e:?}");
        });

        match msg {
            ComponentReplicationMessage::ComponentReplication { replicated, component_id } => {
                waiting_data.0.extend(
                    replicated
                        .into_iter()
                        .flat_map(|c| {
                            repl_comp_data(
                                &q_block_data,
                                &mut network_mapping,
                                &q_structure_systems,
                                &mut q_inventory,
                                &mut commands,
                                &mut q_structure,
                                &mut ev_writer_sync,
                                &mut evw_block_data_changed,
                                component_id,
                                c,
                            )
                        })
                        .map(|repl| (component_id, repl)),
                );
            }
            ComponentReplicationMessage::RemovedComponent {
                component_id,
                entity_identifier,
            } => {
                let (entity, authority_entity) = match get_entity_identifier_info(
                    entity_identifier,
                    &q_block_data,
                    &mut network_mapping,
                    &q_structure_systems,
                    &mut q_inventory,
                    &mut q_structure,
                    &mut evw_block_data_changed,
                    &mut commands,
                ) {
                    Some(value) => value,
                    None => {
                        continue;
                    }
                };

                ev_writer_remove.write(GotComponentToRemoveEvent {
                    // `client_id` only matters on the server-side, but I don't feel like fighting with
                    // my LSP to have this variable only show up in the server project. Thus, I fill it with
                    // dummy data.
                    client_id: 0,
                    component_id,
                    entity,
                    // This also only matters on server-side, but once again I don't care
                    authority_entity,
                });
            }
        }
    }
}

fn repl_comp_data(
    q_block_data: &Query<&BlockData>,
    network_mapping: &mut ResMut<NetworkMapping>,
    q_structure_systems: &Query<&StructureSystems, ()>,
    q_inventory: &mut Query<&mut Inventory, ()>,
    commands: &mut Commands,
    q_structure: &mut Query<&mut Structure>,
    ev_writer_sync: &mut EventWriter<GotComponentToSyncEvent>,
    evw_block_data_changed: &mut EventWriter<BlockDataChangedEvent>,
    component_id: ComponentId,
    c: ReplicatedComponentData,
) -> Option<ReplicatedComponentData> {
    let ReplicatedComponentData {
        entity_identifier,
        raw_data,
    } = c;

    let (entity, authority_entity) = match get_entity_identifier_info(
        entity_identifier,
        q_block_data,
        network_mapping,
        q_structure_systems,
        q_inventory,
        q_structure,
        evw_block_data_changed,
        commands,
    ) {
        Some(value) => value,
        None => {
            return Some(ReplicatedComponentData {
                entity_identifier,
                raw_data,
            });
        }
    };

    let ev = GotComponentToSyncEvent {
        // `client_id` only matters on the server-side, but I don't feel like fighting with
        // my LSP to have this variable only show up in the server project. Thus, I fill it with
        // dummy data.
        client_id: 0,
        component_id,
        entity,
        // This also only matters on server-side, but once again I don't care
        authority_entity,
        raw_data,
    };

    ev_writer_sync.write(ev);
    None
}

fn get_entity_identifier_info(
    entity_identifier: ComponentEntityIdentifier,
    q_block_data: &Query<&BlockData>,
    network_mapping: &mut NetworkMapping,
    q_structure_systems: &Query<&StructureSystems, ()>,
    q_inventory: &mut Query<&mut Inventory>,
    q_structure: &mut Query<&mut Structure>,
    evw_block_data_changed: &mut EventWriter<BlockDataChangedEvent>,
    commands: &mut Commands,
) -> Option<(Entity, Entity)> {
    let identifier_entities = match entity_identifier {
        ComponentEntityIdentifier::Entity(entity) => network_mapping.client_from_server(&entity).map(|x| (x, x)),
        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
            network_mapping.client_from_server(&structure_entity).and_then(|structure_entity| {
                let structure_systems = q_structure_systems.get(structure_entity).ok()?;

                let system_entity = structure_systems.get_system_entity(id)?;

                Some((system_entity, structure_entity))
            })
        }
        ComponentEntityIdentifier::ItemData {
            inventory_entity,
            item_slot,
            server_data_entity,
        } => network_mapping.client_from_server(&inventory_entity).and_then(|inventory_entity| {
            let mut inventory = q_inventory.get_mut(inventory_entity).ok()?;

            // This creates a data entity if it doesn't exist and gets the data entity.
            // TODO: Make this a method to make this less hacky?
            let maybe_data_ent = inventory.insert_itemstack_data(item_slot as usize, (), commands);

            if let Some(de) = maybe_data_ent {
                network_mapping.add_mapping(de, server_data_entity);
            }

            maybe_data_ent.map(|x| (x, x))
        }),
        ComponentEntityIdentifier::BlockData {
            mut identifier,
            server_data_entity,
        } => network_mapping
            .client_from_server(&server_data_entity)
            .map(|x| {
                if !q_block_data.contains(x) {
                    error!("Component got for block data but had no block data component - requesting entity. (Client: {x:?})");

                    return network_mapping
                        .client_from_server(&identifier.block.structure())
                        .and_then(|structure_entity| {
                            let mut structure = q_structure.get_mut(structure_entity).ok()?;
                            let data_entity = structure.get_or_create_block_data_for_block_id(
                                identifier.block.coords(),
                                identifier.block_id,
                                commands,
                            )?;

                            identifier.block.set_structure(structure_entity);

                            network_mapping.add_mapping(data_entity, server_data_entity);

                            evw_block_data_changed.write(BlockDataChangedEvent {
                                block: identifier.block,
                                block_data_entity: Some(data_entity),
                            });

                            Some((data_entity, data_entity))
                        });
                };

                if let Ok(block) = identifier.block.map_to_client(network_mapping) {
                    evw_block_data_changed.write(BlockDataChangedEvent {
                        block,
                        block_data_entity: Some(x),
                    });
                }

                Some((x, x))
            })
            .unwrap_or_else(|| {
                network_mapping
                    .client_from_server(&identifier.block.structure())
                    .and_then(|structure_entity| {
                        // We could either be

                        let mut structure = q_structure.get_mut(structure_entity).ok()?;

                        let coords = identifier.block.coords();
                        let block_id = identifier.block_id;

                        let mut data_entity = structure.get_or_create_block_data_for_block_id(coords, block_id, commands)?;

                        if network_mapping
                            .server_from_client(&data_entity)
                            .map(|x| x != server_data_entity)
                            .unwrap_or(false)
                        {
                            // We have an outdated data entity - remove it.
                            // This happens when the server sending updated data for a new entity
                            // happens before sending the command to despawn the old data entity.
                            commands.entity(data_entity).insert(NeedsDespawned);

                            structure.set_block_data_entity(coords, None);

                            data_entity = structure.get_or_create_block_data_for_block_id(coords, block_id, commands)?;
                        }

                        network_mapping.add_mapping(data_entity, server_data_entity);

                        identifier.block.set_structure(structure_entity);

                        evw_block_data_changed.write(BlockDataChangedEvent {
                            block: identifier.block,
                            block_data_entity: Some(data_entity),
                        });

                        Some((data_entity, data_entity))
                    })
            }),
    };

    if let Some(identifier_entities) = identifier_entities {
        return Some(identifier_entities);
    }

    let (entity, authority_entity) = match entity_identifier {
        ComponentEntityIdentifier::Entity(entity) => {
            let client_entity = commands.spawn((ServerEntity(entity), Name::new("Waiting for server..."))).id();
            network_mapping.add_mapping(client_entity, entity);

            (client_entity, client_entity)
        }
        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
            trace!(
                "Got structure system synced component, but no valid structure exists for it! ({structure_entity:?}, {id:?}). In the future, this should try again once we receive the correct structure from the server."
            );

            return None;
        }
        ComponentEntityIdentifier::ItemData {
            inventory_entity,
            item_slot,
            server_data_entity,
        } => {
            trace!(
                "Got itemdata synced component, but no valid inventory OR itemstack exists for it! ({inventory_entity:?}, {item_slot} {server_data_entity:?})."
            );

            return None;
        }
        ComponentEntityIdentifier::BlockData {
            identifier,
            server_data_entity,
        } => {
            trace!("Got blockdata synced component, but no valid block exists for it! ({identifier:?}, {server_data_entity:?}).");

            return None;
        }
    };

    Some((entity, authority_entity))
}

pub(super) fn setup_client(app: &mut App) {
    app.configure_sets(FixedUpdate, ClientReceiveComponents::ClientReceiveComponents);

    // ComponentSyncingSet configuration in cosmos_client/netty/mod

    app.add_systems(
        FixedUpdate,
        client_receive_components
            .in_set(ClientReceiveComponents::ClientReceiveComponents)
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(resource_exists::<RenetClient>)
            .run_if(resource_exists::<NetworkMapping>),
    )
    .init_resource::<WaitingData>();
}

pub(super) fn sync_component_client<T: SyncableComponent>(app: &mut App) {
    app.register_type::<ServerEntity>();

    if T::get_sync_type().is_client_authoritative() {
        register_fixed_update_removed_component::<T>(app);
        app.add_systems(
            FixedUpdate,
            (client_send_components::<T>, client_send_removed_components::<T>)
                .chain()
                .run_if(resource_exists::<RenetClient>)
                .in_set(ComponentSyncingSet::DoComponentSyncing),
        );
    }
    if T::get_sync_type().is_server_authoritative() {
        add_multi_statebound_resource::<StoredComponents<T>, GameState>(app, GameState::LoadingWorld, GameState::Playing);

        app.add_systems(
            FixedUpdate,
            (
                client_add_stored_components::<T>,
                client_deserialize_component::<T>,
                client_remove_component::<T>,
            )
                .chain()
                .run_if(resource_exists::<NetworkMapping>)
                .run_if(resource_exists::<Registry<SyncedComponentId>>)
                .in_set(ComponentSyncingSet::ReceiveComponents),
        );
    }
}
