use std::marker::PhantomData;

use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::With,
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    log::{error, warn},
    prelude::{BuildChildrenTransformExt, Deref, DerefMut, SystemSet},
    state::condition::in_state,
    utils::HashMap,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::specific_blocks::gravity_well::GravityWell,
    netty::{
        cosmos_encoder, server_replication::ReplicationMessage, sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    physics::location::LocationPhysicsSet,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::{
        loading::StructureLoadingSet,
        systems::{
            StructureSystem, StructureSystemId, StructureSystemImpl, StructureSystemType, StructureSystemTypeId, StructureSystems,
            SystemActive,
        },
    },
};
use serde::{de::DeserializeOwned, Serialize};

use crate::structure::planet::align_player::{self, PlayerAlignment};

#[derive(Event, Debug, Clone)]
struct StructureSystemNeedsUpdated {
    system_id: StructureSystemId,
    structure_entity: Entity,
    system_type_id: StructureSystemTypeId,
    raw: Vec<u8>,
}

#[derive(Resource, Deref, DerefMut, Debug)]
/// Most times structure systems are sent by the server before the structure is actually done loading.
///
/// This is in place to hold those systems until the structure is loaded. Some sort of timer should be added to prevent storing
/// infinite numbers of systems though.
struct SystemsQueue<T> {
    #[deref]
    map: HashMap<Entity, Vec<StructureSystemNeedsUpdated>>,
    _phantom: PhantomData<T>,
}

impl<T> Default for SystemsQueue<T> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            _phantom: Default::default(),
        }
    }
}

fn replication_listen_netty(
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    mut event_writer: EventWriter<StructureSystemNeedsUpdated>,
    q_systems: Query<&StructureSystems>,
    mut commands: Commands,
    q_is_active: Query<(), With<SystemActive>>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::SystemReplication) {
        let msg: ReplicationMessage = cosmos_encoder::deserialize(&message).expect("Unable to parse registry sync from server");

        match msg {
            ReplicationMessage::SystemReplication {
                structure_entity,
                system_id,
                system_type_id,
                raw,
            } => {
                let Some(structure_entity) = mapping.client_from_server(&structure_entity) else {
                    continue;
                };

                event_writer.send(StructureSystemNeedsUpdated {
                    raw,
                    structure_entity,
                    system_id,
                    system_type_id,
                });
            }
            ReplicationMessage::SystemStatus {
                structure_entity,
                system_id,
                active,
            } => {
                let Some(structure_entity) = mapping.client_from_server(&structure_entity) else {
                    continue;
                };

                let Ok(systems) = q_systems.get(structure_entity) else {
                    continue;
                };

                let Some(system) = systems.get_system_entity(system_id) else {
                    warn!("Invalid system id for system {system_id:?} on structure {structure_entity:?}");
                    warn!("{systems:?}");

                    continue;
                };

                if active {
                    if !q_is_active.contains(system) {
                        commands.entity(system).insert(SystemActive);
                    }
                } else if q_is_active.contains(system) {
                    commands.entity(system).remove::<SystemActive>();
                }
            }
            ReplicationMessage::GravityWell { gravity_well, entity } => {
                let Some(entity) = mapping.client_from_server(&entity) else {
                    warn!("Missing entity for gravity well!");
                    continue;
                };

                let Some(mut ecmds) = commands.get_entity(entity) else {
                    continue;
                };

                if let Some(mut grav_well) = gravity_well {
                    let Some(structure_entity) = mapping.client_from_server(&grav_well.structure_entity) else {
                        warn!("Missing structure entity for gravity well!");
                        continue;
                    };

                    grav_well.structure_entity = structure_entity;

                    ecmds
                        .insert((
                            grav_well,
                            PlayerAlignment {
                                axis: align_player::Axis::Y,
                                aligned_to: None,
                            },
                        ))
                        .set_parent_in_place(structure_entity);
                } else {
                    ecmds.remove::<GravityWell>();
                }
            }
        }
    }
}

fn sync<T: StructureSystemImpl + Serialize + DeserializeOwned>(
    system_types: Res<Registry<StructureSystemType>>,
    mut ev_reader: EventReader<StructureSystemNeedsUpdated>,
    mut systems_query: Query<&mut StructureSystems>,
    mut q_system: Query<(&mut T, &StructureSystem)>,
    mut commands: Commands,
    mut sys_queue: ResMut<SystemsQueue<T>>,
) {
    for ev in ev_reader.read() {
        let Some(system_type) = system_types.try_from_numeric_id(ev.system_type_id.into()) else {
            warn!("Missing structure system type {:?}", ev.system_type_id);
            continue;
        };

        if system_type.unlocalized_name() != T::unlocalized_name() {
            continue;
        }

        let entries = sys_queue.entry(ev.structure_entity).or_default();
        if let Some((idx, _)) = entries.iter().enumerate().find(|(_, x)| x.system_id == ev.system_id) {
            entries.remove(idx);
        }

        entries.push(ev.clone());
    }

    for (_, needs_updated) in sys_queue.iter_mut() {
        needs_updated.retain(|ev| {
            let Ok(mut systems) = systems_query.get_mut(ev.structure_entity) else {
                // Structure doesn't have the systems component yet, save these systems to be added later once it has that component.
                // This is normally done within a frame or two.
                return true;
            };

            let Ok(system) = cosmos_encoder::deserialize::<T>(&ev.raw) else {
                error!("Unable to deserialize system type {:?}!", ev.system_type_id);
                return false;
            };

            if let Ok((mut sys, structure_system)) = systems.query_mut(&mut q_system) {
                assert_eq!(
                    structure_system.id(),
                    ev.system_id,
                    "System ids not equal, and multiple systems of the same type aren't supported yet - something went super wrong!"
                );

                *sys = system;
            } else {
                systems.add_system_with_id(&mut commands, system, ev.system_id, &system_types);
            }

            false
        });
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum SystemSyncingSet {
    SyncSystems,
}

pub fn sync_system<T: StructureSystemImpl + Serialize + DeserializeOwned>(app: &mut App) {
    app.configure_sets(Update, SystemSyncingSet::SyncSystems.in_set(NetworkingSystemsSet::SyncComponents));

    app.add_systems(
        Update,
        sync::<T>
            .in_set(SystemSyncingSet::SyncSystems)
            .ambiguous_with(SystemSyncingSet::SyncSystems)
            .run_if(in_state(GameState::Playing))
            .after(replication_listen_netty),
    )
    .init_resource::<SystemsQueue<T>>();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        replication_listen_netty
            .before(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing))
            .after(StructureLoadingSet::StructureLoaded),
    )
    .add_event::<StructureSystemNeedsUpdated>();
}
