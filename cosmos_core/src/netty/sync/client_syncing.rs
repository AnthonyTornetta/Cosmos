use super::mapping::NetworkMapping;
use super::{
    deserialize_component, register_component, ComponentEntityIdentifier, ComponentReplicationMessage, SyncType, SyncableComponent,
    SyncableEntity, SyncedComponentId,
};
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::NettyChannelServer;
use crate::netty::{cosmos_encoder, NettyChannelClient};
use crate::registry::{identifiable::Identifiable, Registry};
use crate::structure::systems::{StructureSystem, StructureSystems};
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::schedule::IntoSystemConfigs;
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
use bevy_renet::renet::RenetClient;

fn client_send_components<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_changed_component: Query<(Entity, &T, Option<&SyncableEntity>, Option<&StructureSystem>), Changed<T>>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
) {
    if q_changed_component.is_empty() {
        return;
    }

    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

    q_changed_component
        .iter()
        .for_each(|(entity, component, syncable_entity, structure_system)| {
            let entity_identifier = match (syncable_entity, structure_system) {
                (None, _) => mapping.server_from_client(&entity).map(|e| ComponentEntityIdentifier::Entity(e)),
                (Some(SyncableEntity::StructureSystem), Some(structure_system)) => mapping
                    .server_from_client(&structure_system.structure_entity())
                    .map(|e| ComponentEntityIdentifier::StructureSystem {
                        structure_entity: e,
                        id: structure_system.id(),
                    }),
                _ => None,
            };

            let Some(entity_identifier) = entity_identifier else {
                warn!("Invalid entity found - {entity_identifier:?} SyncableEntity settings: {syncable_entity:?}");
                return;
            };

            client.send_message(
                NettyChannelClient::ComponentReplication,
                cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                    component_id: id.id(),
                    entity_identifier,
                    // Avoid double compression using bincode instead of cosmos_encoder.
                    raw_data: bincode::serialize(component).expect("Failed to serialize component."),
                }),
            )
        });
}

pub(super) fn client_receive_components(
    mut client: ResMut<RenetClient>,
    mut ev_writer: EventWriter<GotComponentToSyncEvent>,
    mapping: Res<NetworkMapping>,
    q_structure_systems: Query<&StructureSystems>,
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
                    ComponentEntityIdentifier::Entity(entity) => mapping.client_from_server(&entity),
                    ComponentEntityIdentifier::StructureSystem { structure_entity, id } => mapping
                        .client_from_server(&structure_entity)
                        .map(|structure_entity| {
                            let Ok(structure_systems) = q_structure_systems.get(structure_entity) else {
                                return None;
                            };

                            let Some(system_entity) = structure_systems.get_system_entity(id) else {
                                return None;
                            };

                            Some(system_entity)
                        })
                        .flatten(),
                };

                let Some(entity) = entity else {
                    warn!("Missing entity from server: {:?}", entity_identifier);
                    continue;
                };

                ev_writer.send(GotComponentToSyncEvent {
                    component_id,
                    entity,
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

    if T::get_sync_type() != SyncType::ClientAuthoritative {
        app.add_systems(Update, client_send_components::<T>.run_if(resource_exists::<RenetClient>));
    }

    if T::get_sync_type() != SyncType::ServerAuthoritative {
        app.add_systems(
            Update,
            deserialize_component::<T>.run_if(resource_exists::<Registry<SyncedComponentId>>),
        );
    }
}
