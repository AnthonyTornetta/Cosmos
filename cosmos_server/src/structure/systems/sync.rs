use bevy::{
    app::{App, Startup, Update},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::{Added, Changed},
        removal_detection::RemovedComponents,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Query, ResMut},
    },
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_replication::ReplicationMessage, NettyChannelServer},
    registry::Registry,
    structure::systems::{sync::SyncableSystem, StructureSystem, StructureSystemType, SystemActive, Systems},
};

use crate::{netty::sync::entities::RequestedEntityEvent, registry::sync_registry, state::GameState};

fn sync_system<T: SyncableSystem>(mut server: ResMut<RenetServer>, q_changed_systems: Query<(&T, &StructureSystem), Changed<T>>) {
    for (changed_system, structure_system) in q_changed_systems.iter() {
        server.broadcast_message(
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::SystemReplication {
                structure_entity: structure_system.structure_entity(),
                system_id: structure_system.id(),
                system_type_id: structure_system.system_type_id(),
                raw: cosmos_encoder::serialize(changed_system),
            }),
        );
    }
}

fn on_request_systems_entity<T: SyncableSystem>(
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<RequestedEntityEvent>,
    q_systems: Query<&Systems>,
    q_syncable_system: Query<(&T, &StructureSystem)>,
) {
    for ev in ev_reader.read() {
        let Ok(systems) = q_systems.get(ev.entity) else {
            continue;
        };

        let Ok((synacble_system, structure_system)) = systems.query(&q_syncable_system) else {
            continue;
        };

        server.send_message(
            ev.client_id,
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::SystemReplication {
                structure_entity: structure_system.structure_entity(),
                system_id: structure_system.id(),
                system_type_id: structure_system.system_type_id(),
                raw: cosmos_encoder::serialize(synacble_system),
            }),
        );
    }
}

fn sync_active_systems(
    mut server: ResMut<RenetServer>,
    q_structure_system: Query<&StructureSystem>,
    q_active: Query<Entity, Added<SystemActive>>,
    mut q_inactive: RemovedComponents<SystemActive>,
) {
    for active in q_active.iter() {
        let Ok(system) = q_structure_system.get(active) else {
            continue;
        };

        server.broadcast_message(
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::SystemStatus {
                structure_entity: system.structure_entity(),
                system_id: system.id(),
                active: true,
            }),
        );
    }

    for inactive in q_inactive.read() {
        let Ok(system) = q_structure_system.get(inactive) else {
            continue;
        };

        server.broadcast_message(
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::SystemStatus {
                structure_entity: system.structure_entity(),
                system_id: system.id(),
                active: false,
            }),
        );
    }
}

fn register_system<T: SyncableSystem>(mut registry: ResMut<Registry<StructureSystemType>>) {
    registry.register(StructureSystemType::new(T::unlocalized_name()));
}

pub fn register_structure_system<T: SyncableSystem>(app: &mut App) {
    app.add_systems(Startup, register_system::<T>).add_systems(
        Update,
        (sync_system::<T>, sync_active_systems, on_request_systems_entity::<T>).run_if(in_state(GameState::Playing)),
    );
}

pub(super) fn register(app: &mut App) {
    sync_registry::<StructureSystemType>(app);
}
