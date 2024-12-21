//! Exposes [`ClientReceiveComponents::ClientReceiveComponents`] - this is to remove ambiguity

use super::mapping::{NetworkMapping, ServerEntity};
use super::{
    ClientAuthority, ComponentEntityIdentifier, ComponentReplicationMessage, ComponentSyncingSet, GotComponentToRemoveEvent,
    ReplicatedComponentData, SyncType, SyncableComponent, SyncedComponentId,
};
use crate::block::data::BlockData;
use crate::ecs::NeedsDespawned;
use crate::events::block_events::BlockDataChangedEvent;
use crate::inventory::itemstack::ItemStackData;
use crate::inventory::Inventory;
use crate::netty::client::{LocalPlayer, NeedsLoadedFromServer};
use crate::netty::client_reliable_messages::ClientReliableMessages;
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::system_sets::NetworkingSystemsSet;
use crate::netty::{cosmos_encoder, NettyChannelClient};
use crate::netty::{NettyChannelServer, NoSendEntity};
use crate::registry::{identifiable::Identifiable, Registry};
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use crate::structure::Structure;
use bevy::core::Name;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::removal_detection::RemovedComponents;
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::ecs::system::{Commands, Resource};
use bevy::log::warn;
use bevy::prelude::SystemSet;
use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::EventWriter,
        query::Changed,
        system::{Query, Res, ResMut},
    },
    log::error,
};
use bevy_renet2::renet2::{ClientId, RenetClient};

fn client_deserialize_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToSyncEvent>,
    mut commands: Commands,
    mapping: Res<NetworkMapping>,
    q_t: Query<&T>,
) {
    for ev in ev_reader.read() {
        let synced_id = components_registry
            .try_from_numeric_id(ev.component_id)
            .unwrap_or_else(|| panic!("Missing component with id {}\n\n{components_registry:?}\n\n", ev.component_id));

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        if let Some(mut ecmds) = commands.get_entity(ev.entity) {
            let mut component = bincode::deserialize::<T>(&ev.raw_data).expect("Failed to deserialize component sent from server!");

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
                ecmds.try_insert(component);
            }
        } else {
            warn!("No entity cmds for synced entity component - (entity {:?})", ev.entity);
        }
    }
}

fn client_remove_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToRemoveEvent>,
    mut commands: Commands,
) {
    for ev in ev_reader.read() {
        let synced_id = components_registry
            .try_from_numeric_id(ev.component_id)
            .unwrap_or_else(|| panic!("Missing component with id {}", ev.component_id));

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        if let Some(mut ecmds) = commands.get_entity(ev.entity) {
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
            let entity_identifier = compute_entity_identifier(structure_system, &mapping, is_data, entity);

            let Some((entity_identifier, authority_entity)) = entity_identifier else {
                warn!("Invalid entity found - {entity_identifier:?} - no match on server entities!");
                return None;
            };

            // The server checks this anyway, but save the client+server some bandwidth
            match authority {
                ClientAuthority::Anything => {}
                ClientAuthority::Piloting => {
                    let Ok(piloting) = q_local_piloting.get_single() else {
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
                let mapped = component.clone().convert_entities_client_to_server(&mapping)?;

                bincode::serialize(&mapped)
            } else {
                bincode::serialize(component)
            }
            .expect("Failed to serialize component.");

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
                component_id: id.id(),
                replicated: data_to_sync,
            }),
        );
    }
}

fn client_send_removed_components<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    mut removed_components: RemovedComponents<T>,
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
                let Ok(piloting) = q_local_piloting.get_single() else {
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
                component_id: id.id(),
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
struct WaitingData(Vec<(u16, ReplicatedComponentData)>);

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
            &mut client,
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
                                &mut client,
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
                    &mut client,
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

                ev_writer_remove.send(GotComponentToRemoveEvent {
                    // `client_id` only matters on the server-side, but I don't feel like fighting with
                    // my LSP to have this variable only show up in the server project. Thus, I fill it with
                    // dummy data.
                    client_id: ClientId::from_raw(0),
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
    client: &mut RenetClient,
    q_block_data: &Query<&BlockData>,
    network_mapping: &mut ResMut<NetworkMapping>,
    q_structure_systems: &Query<&StructureSystems, ()>,
    q_inventory: &mut Query<&mut Inventory, ()>,
    commands: &mut Commands,
    q_structure: &mut Query<&mut Structure>,
    ev_writer_sync: &mut EventWriter<GotComponentToSyncEvent>,
    evw_block_data_changed: &mut EventWriter<BlockDataChangedEvent>,
    component_id: u16,
    c: ReplicatedComponentData,
) -> Option<ReplicatedComponentData> {
    let ReplicatedComponentData {
        entity_identifier,
        raw_data,
    } = c;

    let (entity, authority_entity) = match get_entity_identifier_info(
        client,
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
        client_id: ClientId::from_raw(0),
        component_id,
        entity,
        // This also only matters on server-side, but once again I don't care
        authority_entity,
        raw_data,
    };

    ev_writer_sync.send(ev);
    None
}

fn get_entity_identifier_info(
    client: &mut RenetClient,
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

                    client.send_message(
                        NettyChannelClient::Reliable,
                        cosmos_encoder::serialize(&ClientReliableMessages::RequestEntityData {
                            entity: server_data_entity,
                        }),
                    );

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

                            evw_block_data_changed.send(BlockDataChangedEvent {
                                block: identifier.block,
                                block_data_entity: Some(data_entity),
                            });

                            Some((data_entity, data_entity))
                        });
                };

                evw_block_data_changed.send(BlockDataChangedEvent {
                    block: identifier.block,
                    block_data_entity: Some(x),
                });

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

                        evw_block_data_changed.send(BlockDataChangedEvent {
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
            let client_entity = commands
                .spawn((ServerEntity(entity), NeedsLoadedFromServer, Name::new("Waiting for server...")))
                .id();
            network_mapping.add_mapping(client_entity, entity);

            (client_entity, client_entity)
        }
        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
            warn!(
                    "Got structure system synced component, but no valid structure exists for it! ({structure_entity:?}, {id:?}). In the future, this should try again once we receive the correct structure from the server."
                );

            return None;
        }
        ComponentEntityIdentifier::ItemData {
            inventory_entity,
            item_slot,
            server_data_entity,
        } => {
            warn!(
                "Got itemdata synced component, but no valid inventory OR itemstack exists for it! ({inventory_entity:?}, {item_slot} {server_data_entity:?})."
            );

            return None;
        }
        ComponentEntityIdentifier::BlockData {
            identifier,
            server_data_entity,
        } => {
            warn!("Got blockdata synced component, but no valid block exists for it! ({identifier:?}, {server_data_entity:?}).");

            return None;
        }
    };

    Some((entity, authority_entity))
}

pub(super) fn setup_client(app: &mut App) {
    app.configure_sets(Update, ClientReceiveComponents::ClientReceiveComponents);

    // ComponentSyncingSet configuration in cosmos_client/netty/mod

    app.add_systems(
        Update,
        client_receive_components
            .in_set(ClientReceiveComponents::ClientReceiveComponents)
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .run_if(resource_exists::<RenetClient>)
            .run_if(resource_exists::<NetworkMapping>),
    )
    .init_resource::<WaitingData>();
}

#[allow(unused)] // This function is used, but the LSP can't figure that out.
pub(super) fn sync_component_client<T: SyncableComponent>(app: &mut App) {
    app.register_type::<ServerEntity>();

    match T::get_sync_type() {
        SyncType::ClientAuthoritative(_) => {
            app.add_systems(
                Update,
                (client_send_components::<T>, client_send_removed_components::<T>)
                    .chain()
                    .run_if(resource_exists::<RenetClient>)
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
        }
        SyncType::ServerAuthoritative => {
            app.add_systems(
                Update,
                (client_deserialize_component::<T>, client_remove_component::<T>)
                    .chain()
                    .run_if(resource_exists::<NetworkMapping>)
                    .run_if(resource_exists::<Registry<SyncedComponentId>>)
                    .in_set(ComponentSyncingSet::ReceiveComponents),
            );
        }
        SyncType::BothAuthoritative(_) => {
            app.add_systems(
                Update,
                (client_send_components::<T>, client_send_removed_components::<T>)
                    .chain()
                    .run_if(resource_exists::<RenetClient>)
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
            app.add_systems(
                Update,
                (client_deserialize_component::<T>, client_remove_component::<T>)
                    .chain()
                    .run_if(resource_exists::<NetworkMapping>)
                    .run_if(resource_exists::<Registry<SyncedComponentId>>)
                    .in_set(ComponentSyncingSet::ReceiveComponents),
            );
        }
    }
}
