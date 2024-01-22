use bevy::{
    app::{App, Startup, Update},
    ecs::{
        query::Changed,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Query, ResMut},
    },
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_replication::ReplicationMessage, NettyChannelServer},
    registry::Registry,
    structure::systems::{sync::SyncableSystem, StructureSystem, StructureSystemType},
};

use crate::{registry::sync_registry, state::GameState};

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

fn register_system<T: SyncableSystem>(mut registry: ResMut<Registry<StructureSystemType>>) {
    registry.register(StructureSystemType::new(T::unlocalized_name()));
}

pub fn register_structure_system<T: SyncableSystem>(app: &mut App) {
    app.add_systems(Startup, register_system::<T>)
        .add_systems(Update, sync_system::<T>.run_if(in_state(GameState::Playing)));
}

pub(super) fn register(app: &mut App) {
    sync_registry::<StructureSystemType>(app);
}
