use super::mapping::NetworkMapping;
use super::{deserialize_component, register_component, ComponentReplicationMessage, SyncType, SyncableComponent, SyncedComponentId};
use crate::netty::sync::GotComponentToSyncEvent;
use crate::netty::{cosmos_encoder, NettyChannelServer};
use crate::registry::{identifiable::Identifiable, Registry};
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
use bevy_renet::renet::RenetServer;

fn server_send_component<T: SyncableComponent>(
    id_registry: Res<Registry<SyncedComponentId>>,
    q_changed_component: Query<(Entity, &T), Changed<T>>,
    mut server: ResMut<RenetServer>,
) {
    if q_changed_component.is_empty() {
        return;
    }

    let Some(id) = id_registry.from_id(T::get_component_unlocalized_name()) else {
        error!("Invalid component unlocalized name - {}", T::get_component_unlocalized_name());
        return;
    };

    q_changed_component.iter().for_each(|(entity, component)| {
        server.broadcast_message(
            NettyChannelServer::ComponentReplication,
            cosmos_encoder::serialize(&ComponentReplicationMessage::ComponentReplication {
                component_id: id.id(),
                entity,
                // Avoid double compression using bincode instead of cosmos_encoder.
                raw_data: bincode::serialize(component).expect("Failed to serialize component."),
            }),
        )
    });
}

#[cfg(feature = "server")]
pub(super) fn server_receive_components(
    mut server: ResMut<RenetServer>,
    mut ev_writer: EventWriter<GotComponentToSyncEvent>,
    mapping: Res<NetworkMapping>,
) {
    use crate::netty::NettyChannelClient;

    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::ComponentReplication) {
            let Ok(msg) = cosmos_encoder::deserialize::<ComponentReplicationMessage>(&message) else {
                continue;
            };

            match msg {
                ComponentReplicationMessage::ComponentReplication {
                    component_id,
                    entity,
                    raw_data,
                } => {
                    let Some(entity) = mapping.client_from_server(&entity) else {
                        warn!("Missing entity from server: {:?}", entity);
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
}

#[allow(unused)] // This function is used, but the LSP can't figure that out.
pub(super) fn sync_component_server<T: SyncableComponent>(app: &mut App) {
    app.add_systems(Startup, register_component::<T>);

    if T::get_sync_type() != SyncType::ServerAuthoritative {
        app.add_systems(Update, server_send_component::<T>);
    }

    if T::get_sync_type() != SyncType::ClientAuthoritative {
        app.add_systems(Update, deserialize_component::<T>);
    }
}
