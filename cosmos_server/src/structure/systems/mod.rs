//! Contains projectile systems needed on the server

use bevy::{
    app::Update,
    ecs::{
        entity::Entity,
        query::Added,
        removal_detection::RemovedComponents,
        system::{Query, ResMut},
    },
    prelude::App,
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{cosmos_encoder, server_reliable_messages::ServerReliableMessages, NettyChannelServer},
    structure::{
        structure_block::StructureBlock,
        systems::{StructureSystem, SystemActive},
    },
};

mod energy_generation_system;
mod energy_storage_system;
mod laser_cannon_system;
mod line_system;
mod mining_laser_system;
pub(crate) mod sync;
mod thruster_system;

fn sync_active_systems(
    q_systems: Query<&StructureSystem>,
    q_activated: Query<Entity, Added<SystemActive>>,
    mut q_deactivated: RemovedComponents<SystemActive>,

    mut server: ResMut<RenetServer>,
) {
    for activated_system in q_activated.iter() {
        let Ok(structure_system) = q_systems.get(activated_system) else {
            continue;
        };

        println!("Sending activated system {:?}!", structure_system.id());

        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::StructureSystemActiveChange {
                system_id: structure_system.id(),
                structure_entity: structure_system.structure_entity(),
                active: true,
            }),
        );
    }

    for deactivated_system in q_deactivated.read() {
        let Ok(structure_system) = q_systems.get(deactivated_system) else {
            continue;
        };

        println!("Sending deactivated system {:?}!", structure_system.id());

        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::StructureSystemActiveChange {
                system_id: structure_system.id(),
                structure_entity: structure_system.structure_entity(),
                active: false,
            }),
        );
    }
}

/// A system that is created by the addition and removal of blocks
pub trait BlockStructureSystem<T> {
    /// Called whenever a block is added that is relevant to this system
    fn add_block(&mut self, sb: &StructureBlock, property: &T);
    /// Called whenever a block is removed that is relevant to this system
    fn remove_block(&mut self, sb: &StructureBlock);
}

pub(super) fn register(app: &mut App) {
    sync::register(app);
    line_system::register(app);
    laser_cannon_system::register(app);
    thruster_system::register(app);
    energy_generation_system::register(app);
    mining_laser_system::register(app);
    energy_storage_system::register(app);

    app.add_systems(Update, sync_active_systems);
}
