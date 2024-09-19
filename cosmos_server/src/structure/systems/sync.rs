use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::{Added, Changed, Without},
        removal_detection::RemovedComponents,
        schedule::IntoSystemConfigs,
        system::{Query, Res, ResMut},
    },
    state::{condition::in_state, state::OnExit},
};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    item::Item,
    netty::{
        cosmos_encoder,
        server_replication::ReplicationMessage,
        sync::{registry::sync_registry, server_entity_syncing::RequestedEntityEvent},
        NettyChannelServer, NoSendEntity,
    },
    registry::{identifiable::Identifiable, Registry},
    structure::systems::{sync::SyncableSystem, StructureSystem, StructureSystemType, StructureSystems, StructureSystemsSet, SystemActive},
};

use crate::state::GameState;

fn sync_system<T: SyncableSystem>(
    mut server: ResMut<RenetServer>,
    q_changed_systems: Query<(&T, &StructureSystem), (Without<NoSendEntity>, Changed<T>)>,
) {
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
    q_systems: Query<&StructureSystems>,
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

pub fn register_structure_system<T: SyncableSystem>(app: &mut App, activatable: bool, item_icon_unlocalized_name: impl Into<String>) {
    let item_icon_unlocalized_name = item_icon_unlocalized_name.into();

    app.add_systems(
        Update,
        (sync_system::<T>, sync_active_systems, on_request_systems_entity::<T>)
            .after(StructureSystemsSet::UpdateSystems)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        OnExit(GameState::PostLoading),
        move |items: Res<Registry<Item>>, mut registry: ResMut<Registry<StructureSystemType>>| {
            let Some(item) = items.from_id(&item_icon_unlocalized_name) else {
                panic!("Could not find item with id {}", item_icon_unlocalized_name);
            };

            registry.register(StructureSystemType::new(T::unlocalized_name(), activatable, item.id()));
        },
    );
}

pub(super) fn register(app: &mut App) {}
