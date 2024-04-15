use super::mapping::NetworkMapping;
use super::{
    register_component, ClientAuthority, ComponentEntityIdentifier, ComponentReplicationMessage, ComponentSyncingSet, SyncType,
    SyncableComponent, SyncedComponentId,
};
use crate::netty::client::LocalPlayer;
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::NettyChannelServer;
use crate::netty::{cosmos_encoder, NettyChannelClient};
use crate::registry::{identifiable::Identifiable, Registry};
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use bevy::core::Name;
use bevy::ecs::event::EventReader;
use bevy::ecs::query::With;
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::ecs::system::Commands;
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
) {
    for ev in ev_reader.read() {
        let synced_id = components_registry
            .try_from_numeric_id(ev.component_id)
            .unwrap_or_else(|| panic!("Missing component with id {}", ev.component_id));

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        if let Some(mut ecmds) = commands.get_entity(ev.entity) {
            let mut component = bincode::deserialize::<T>(&ev.raw_data).expect("Failed to deserialize component sent from server!");
            if T::needs_entity_conversion() {
                let Some(mapped) = component.convert_entities_server_to_client(&mapping) else {
                    continue;
                };

                component = mapped;
            }
            ecmds.try_insert(component);
        }
    }
}

fn client_send_components<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_changed_component: Query<(Entity, &T, Option<&StructureSystem>), Changed<T>>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_local_piloting: Query<&Pilot, With<LocalPlayer>>,
) {
    let SyncType::ClientAuthoritative(authority) = T::get_sync_type() else {
        unreachable!("This function cannot be caled if synctype != client authoritative.")
    };

    if q_changed_component.is_empty() {
        return;
    }
    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

    q_changed_component.iter().for_each(|(entity, component, structure_system)| {
        let entity_identifier = match structure_system {
            None => mapping
                .server_from_client(&entity)
                .map(|e| (ComponentEntityIdentifier::Entity(e), entity)),
            Some(structure_system) => mapping.server_from_client(&structure_system.structure_entity()).map(|e| {
                (
                    ComponentEntityIdentifier::StructureSystem {
                        structure_entity: e,
                        id: structure_system.id(),
                    },
                    structure_system.structure_entity(),
                )
            }),
        };

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

pub(super) fn client_receive_components(
    mut client: ResMut<RenetClient>,
    mut ev_writer: EventWriter<GotComponentToSyncEvent>,
    q_structure_systems: Query<&StructureSystems>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut commands: Commands,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::ComponentReplication) {
        let msg: ComponentReplicationMessage =
            cosmos_encoder::deserialize(&message).expect("Failed to parse component replication message from server!");

        match msg {
            ComponentReplicationMessage::ComponentReplication {
                component_id,
                entity_identifier,
                raw_data,
            } => {
                let entity = match entity_identifier {
                    ComponentEntityIdentifier::Entity(entity) => network_mapping.client_from_server(&entity).map(|x| (x, x)),
                    ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
                        network_mapping.client_from_server(&structure_entity).and_then(|structure_entity| {
                            let structure_systems = q_structure_systems.get(structure_entity).ok()?;

                            let system_entity = structure_systems.get_system_entity(id)?;

                            Some((system_entity, structure_entity))
                        })
                    }
                };

                let (entity, authority_entity) = if let Some(entity) = entity {
                    entity
                } else {
                    match entity_identifier {
                        ComponentEntityIdentifier::Entity(entity) => {
                            let client_entity = commands.spawn(Name::new("Waiting for server...")).id();
                            network_mapping.add_mapping(client_entity, entity);

                            (client_entity, client_entity)
                        }
                        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
                            warn!(
                                "Got structure system synced component, but no valid structure exists for it! ({structure_entity:?}, {id:?}). In the future, this should try again once we receive the correct structure from the server."
                            );

                            continue;
                        }
                    }
                };

                ev_writer.send(GotComponentToSyncEvent {
                    // `client_id` only matters on the server-side, but I don't feel like fighting with
                    // my LSP to have this variable only show up in the server project. Thus, I fill it with
                    // dummy data.
                    client_id: ClientId::from_raw(0),
                    component_id,
                    entity,
                    // This also only matters on server-side, but once again I don't care
                    authority_entity,
                    raw_data,
                });
            }
        }
    }
}

pub(super) fn setup_client(app: &mut App) {
    app.add_systems(
        Update,
        client_receive_components
            .run_if(resource_exists::<RenetClient>)
            .run_if(resource_exists::<NetworkMapping>),
    );
}

#[allow(unused)] // This function is used, but the LSP can't figure that out.
pub(super) fn sync_component_client<T: SyncableComponent>(app: &mut App) {
    app.add_systems(Startup, register_component::<T>);

    match T::get_sync_type() {
        SyncType::ClientAuthoritative(_) => {
            app.add_systems(
                Update,
                client_send_components::<T>
                    .run_if(resource_exists::<RenetClient>)
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
        }
        SyncType::ServerAuthoritative => {
            app.add_systems(
                Update,
                client_deserialize_component::<T>
                    .run_if(resource_exists::<NetworkMapping>)
                    .run_if(resource_exists::<Registry<SyncedComponentId>>)
                    .in_set(ComponentSyncingSet::DoComponentSyncing),
            );
        }
    }
}
