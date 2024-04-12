use super::{
    register_component, ClientAuthority, ComponentEntityIdentifier, ComponentReplicationMessage, SyncType, SyncableComponent,
    SyncableEntity, SyncedComponentId,
};
use crate::netty::server::ServerLobby;
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::{cosmos_encoder, NettyChannelClient, NettyChannelServer};
use crate::registry::{identifiable::Identifiable, Registry};
use crate::structure::ship::pilot::Pilot;
use crate::structure::systems::{StructureSystem, StructureSystems};
use bevy::ecs::event::EventReader;
use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::ecs::schedule::IntoSystemConfigs;
use bevy::ecs::system::Commands;
use bevy::log::{info, warn};
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
use bevy_renet::renet::RenetServer;

fn server_deserialize_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToSyncEvent>,
    mut commands: Commands,
    lobby: Res<ServerLobby>,
    q_piloting: Query<&Pilot>,
) {
    for ev in ev_reader.read() {
        let Some(synced_id) = components_registry.try_from_numeric_id(ev.component_id) else {
            warn!("Missing component with id {}", ev.component_id);
            continue;
        };

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        let SyncType::ClientAuthoritative(authority) = T::get_sync_type() else {
            unreachable!("This function cannot be caled if synctype != client authoritative in the server project.")
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

                if piloting.entity != ev.entity {
                    return;
                }
            }
            ClientAuthority::Themselves => {
                let Some(player) = lobby.player_from_id(ev.client_id) else {
                    return;
                };

                if player != ev.entity {
                    return;
                }
            }
        }

        commands
            .entity(ev.entity)
            .try_insert(bincode::deserialize::<T>(&ev.raw_data).expect("Failed to deserialize component sent from server!"));
    }
}

fn server_send_component<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_changed_component: Query<(Entity, &T, Option<&SyncableEntity>, Option<&StructureSystem>), Changed<T>>,
    mut server: ResMut<RenetServer>,
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
                (None, _) => Some(ComponentEntityIdentifier::Entity(entity)),
                (Some(SyncableEntity::StructureSystem), Some(structure_system)) => Some(ComponentEntityIdentifier::StructureSystem {
                    structure_entity: structure_system.structure_entity(),
                    id: structure_system.id(),
                }),
                _ => None,
            };

            let Some(entity_identifier) = entity_identifier else {
                warn!("Invalid entity found - {entity_identifier:?} SyncableEntity settings: {syncable_entity:?}");
                return;
            };

            server.broadcast_message(
                NettyChannelServer::ComponentReplication,
                cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                    component_id: id.id(),
                    entity_identifier,
                    // Avoid double compression using bincode instead of cosmos_encoder.
                    raw_data: bincode::serialize(component).expect("Failed to serialize component."),
                }),
            )
        });
}

fn server_receive_components(
    mut server: ResMut<RenetServer>,
    mut ev_writer: EventWriter<GotComponentToSyncEvent>,
    q_structure_systems: Query<&StructureSystems>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::ComponentReplication) {
            let Ok(msg) = cosmos_encoder::deserialize::<ComponentReplicationMessage>(&message) else {
                warn!("Bad deserialization");
                continue;
            };

            match msg {
                ComponentReplicationMessage::ComponentReplication {
                    component_id,
                    entity_identifier,
                    raw_data,
                } => {
                    let entity = match entity_identifier {
                        ComponentEntityIdentifier::Entity(entity) => entity,
                        ComponentEntityIdentifier::StructureSystem { structure_entity, id } => {
                            let Ok(structure_systems) = q_structure_systems.get(structure_entity) else {
                                warn!("Bad structure entity {structure_entity:?}");
                                continue;
                            };

                            let Some(system_entity) = structure_systems.get_system_entity(id) else {
                                warn!("Bad structure system id {id:?}");
                                continue;
                            };

                            system_entity
                        }
                    };

                    info!("Syncing component from client!");

                    ev_writer.send(GotComponentToSyncEvent {
                        client_id,
                        component_id,
                        entity,
                        raw_data,
                    });
                }
            }
        }
    }
}

pub(super) fn setup_server(app: &mut App) {
    app.add_systems(Update, server_receive_components);
}

#[allow(unused)] // This function is used, but the LSP can't figure that out.
pub(super) fn sync_component_server<T: SyncableComponent>(app: &mut App) {
    app.add_systems(Startup, register_component::<T>);

    match T::get_sync_type() {
        SyncType::ServerAuthoritative => {
            app.add_systems(Update, server_send_component::<T>.run_if(resource_exists::<RenetServer>));
        }
        SyncType::ClientAuthoritative(_) => {
            app.add_systems(Update, server_deserialize_component::<T>);
        }
    }
}
