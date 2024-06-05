use super::mapping::{NetworkMapping, ServerEntity};
use super::{
    register_component, ClientAuthority, ComponentEntityIdentifier, ComponentReplicationMessage, ComponentSyncingSet,
    GotComponentToRemoveEvent, SyncType, SyncableComponent, SyncedComponentId,
};
use crate::events::block_events::BlockDataChangedEvent;
use crate::inventory::itemstack::ItemStackData;
use crate::inventory::Inventory;
use crate::netty::client::LocalPlayer;
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::{cosmos_encoder, NettyChannelClient};
use crate::netty::{NettyChannelServer, NoSendEntity};
use crate::physics::location::CosmosBundleSet;
use crate::registry::{identifiable::Identifiable, Registry};
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use crate::structure::Structure;
use bevy::core::Name;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::removal_detection::RemovedComponents;
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs};
use bevy::ecs::system::{Commands, Resource};
use bevy::log::warn;
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
use bevy_renet::renet::{ClientId, RenetClient};

fn client_deserialize_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToSyncEvent>,
    mut commands: Commands,
    mapping: Res<NetworkMapping>,
    q_t: Query<&T>,
) {
    for ev in ev_reader.read() {
        println!("Got event!");
        let synced_id = components_registry
            .try_from_numeric_id(ev.component_id)
            .unwrap_or_else(|| panic!("Missing component with id {}", ev.component_id));

        println!("{} != {}???", T::get_component_unlocalized_name(), synced_id.unlocalized_name);
        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            println!("NOT EQUAL!");
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
                println!("Inserting {component:?}");
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

    q_changed_component
        .iter()
        .for_each(|(entity, component, structure_system, is_data)| {
            let entity_identifier = compute_entity_identifier(structure_system, &mapping, is_data, entity);

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
                    if !q_local_player.contains(entity) {
                        return;
                    }
                }
            }

            let raw_data = if T::needs_entity_conversion() {
                let mapped = component.clone().convert_entities_client_to_server(&mapping);

                let Some(mapped) = mapped else {
                    return;
                };

                bincode::serialize(&mapped)
            } else {
                bincode::serialize(component)
            }
            .expect("Failed to serialize component.");

            client.send_message(
                NettyChannelClient::ComponentReplication,
                cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                    component_id: id.id(),
                    entity_identifier,
                    // Avoid double compression using bincode instead of cosmos_encoder.
                    raw_data,
                }),
            )
        });
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
struct WaitingData(Vec<ComponentReplicationMessage>);

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
) {
    let mut v = Vec::with_capacity(waiting_data.0.len());
    std::mem::swap(&mut v, &mut waiting_data.0);
    for msg in v {
        if let Some(msg) = handle_incoming_component_data(
            msg.clone(),
            &mut network_mapping,
            &q_structure_systems,
            &mut q_inventory,
            &mut commands,
            &mut q_structure,
            &mut ev_writer_sync,
            &mut ev_writer_remove,
            &mut evw_block_data_changed,
        ) {
            waiting_data.0.push(msg);
        } else {
            println!("Handled: {msg:?}");
        }
    }

    while let Some(message) = client.receive_message(NettyChannelServer::ComponentReplication) {
        let msg: ComponentReplicationMessage = cosmos_encoder::deserialize(&message).unwrap_or_else(|e| {
            panic!("Failed to parse component replication message from server! Bytes:\n{message:?}\nError: {e:?}");
        });

        if let Some(msg) = handle_incoming_component_data(
            msg,
            &mut network_mapping,
            &q_structure_systems,
            &mut q_inventory,
            &mut commands,
            &mut q_structure,
            &mut ev_writer_sync,
            &mut ev_writer_remove,
            &mut evw_block_data_changed,
        ) {
            waiting_data.0.push(msg);
        }
    }
}

fn handle_incoming_component_data(
    msg: ComponentReplicationMessage,
    network_mapping: &mut ResMut<NetworkMapping>,
    q_structure_systems: &Query<&StructureSystems, ()>,
    q_inventory: &mut Query<&mut Inventory, ()>,
    commands: &mut Commands,
    q_structure: &mut Query<&mut Structure>,
    ev_writer_sync: &mut EventWriter<GotComponentToSyncEvent>,
    ev_writer_remove: &mut EventWriter<GotComponentToRemoveEvent>,
    evw_block_data_changed: &mut EventWriter<BlockDataChangedEvent>,
) -> Option<ComponentReplicationMessage> {
    match msg {
        ComponentReplicationMessage::ComponentReplication {
            component_id,
            entity_identifier,
            raw_data,
        } => {
            let (entity, authority_entity) = match get_entity_identifier_info(
                entity_identifier,
                network_mapping,
                q_structure_systems,
                q_inventory,
                q_structure,
                evw_block_data_changed,
                commands,
            ) {
                Some(value) => value,
                None => {
                    return Some(ComponentReplicationMessage::ComponentReplication {
                        component_id,
                        entity_identifier,
                        raw_data,
                    })
                }
            };

            println!("Sending `GotComponentToSyncEvent` event!");

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

            println!("{ev:?}");

            ev_writer_sync.send(ev);

            None
        }
        ComponentReplicationMessage::RemovedComponent {
            component_id,
            entity_identifier,
        } => {
            let (entity, authority_entity) = match get_entity_identifier_info(
                entity_identifier,
                network_mapping,
                q_structure_systems,
                q_inventory,
                q_structure,
                evw_block_data_changed,
                commands,
            ) {
                Some(value) => value,
                None => {
                    return Some(ComponentReplicationMessage::RemovedComponent {
                        component_id,
                        entity_identifier,
                    })
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

            None
        }
    }
}

fn get_entity_identifier_info(
    entity_identifier: ComponentEntityIdentifier,
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
            println!("Inventory entity should be - {inventory_entity:?}");
            let maybe_data_ent = inventory.insert_itemstack_data(inventory_entity, item_slot as usize, (), commands);

            if let Some(de) = maybe_data_ent {
                network_mapping.add_mapping(de, server_data_entity);
            }

            maybe_data_ent.map(|x| (x, x))
        }),
        ComponentEntityIdentifier::BlockData {
            identifier,
            server_data_entity,
        } => network_mapping
            .client_from_server(&server_data_entity)
            .map(|x| {
                println!("Found matching entity {x:?}!");
                commands.entity(x).log_components();
                Some((x, x))
            })
            .unwrap_or_else(|| {
                network_mapping
                    .client_from_server(&identifier.structure_entity)
                    .and_then(|structure_entity| {
                        let mut structure = q_structure.get_mut(structure_entity).ok()?;
                        let data_entity = structure.get_or_create_block_data(identifier.block.coords(), commands)?;

                        println!("Got block data! server {:?} -> client {data_entity:?}", server_data_entity);

                        network_mapping.add_mapping(data_entity, server_data_entity);

                        evw_block_data_changed.send(BlockDataChangedEvent {
                            block: identifier.block,
                            block_data_entity: Some(data_entity),
                            structure_entity,
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
                "Got itemdata synced component, but no valid inventory OR itemstack exists for it! ({inventory_entity:?}, {item_slot} {server_data_entity:?}). In the future, this should try again once we receive the correct inventory from the server."
            );

            return None;
        }
        ComponentEntityIdentifier::BlockData {
            identifier,
            server_data_entity,
        } => {
            warn!(
                "Got blockdata synced component, but no valid block exists for it! ({identifier:?}, {server_data_entity:?}). In the future, this should try again once we receive the correct inventory from the server."
            );

            return None;
        }
    };

    Some((entity, authority_entity))
}

pub(super) fn setup_client(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ComponentSyncingSet::PreComponentSyncing,
            ComponentSyncingSet::DoComponentSyncing,
            ComponentSyncingSet::PostComponentSyncing,
        )
            .before(CosmosBundleSet::HandleCosmosBundles)
            .chain(),
    );

    app.add_systems(
        Update,
        client_receive_components
            .run_if(resource_exists::<RenetClient>)
            .run_if(resource_exists::<NetworkMapping>),
    )
    .init_resource::<WaitingData>();
}

#[allow(unused)] // This function is used, but the LSP can't figure that out.
pub(super) fn sync_component_client<T: SyncableComponent>(app: &mut App) {
    app.add_systems(Startup, register_component::<T>);

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
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
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
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
        }
    }
}
