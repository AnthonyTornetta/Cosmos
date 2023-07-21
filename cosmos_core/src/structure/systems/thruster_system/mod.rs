//! Thruster block system

use std::ops::Mul;

use bevy::{
    prelude::{
        in_state, App, Commands, Component, EventReader, IntoSystemConfigs, OnEnter, Quat, Query, Res, ResMut, Resource, States, Transform,
        Update, Vec3, With,
    },
    reflect::Reflect,
    time::Time,
    utils::HashMap,
};
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, Velocity};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        events::StructureLoadedEvent,
        ship::{pilot::Pilot, ship_movement::ShipMovement},
        systems::energy_storage_system::EnergyStorageSystem,
        Structure,
    },
};

use super::{StructureSystem, Systems};

const MAX_SHIP_SPEED: f32 = 200.0;
const MAX_BRAKE_DELTA_PER_THRUST: f32 = 300.0;

/// A block that is a thruster will have a thruster property
pub struct ThrusterProperty {
    /// How much thrust this block generates
    pub strength: f32,
    /// How much energy this block consumes
    pub energy_consupmtion: f32,
}

#[derive(Default, Resource)]
struct ThrusterBlocks {
    blocks: HashMap<u16, ThrusterProperty>,
}

impl ThrusterBlocks {
    pub fn insert(&mut self, block: &Block, thruster: ThrusterProperty) {
        self.blocks.insert(block.id(), thruster);
    }

    pub fn get(&self, block: &Block) -> Option<&ThrusterProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Component, Default, Reflect)]
/// Represents all the thruster blocks on this structure
pub struct ThrusterSystem {
    thrust_total: f32,
    energy_consumption: f32,
}

impl ThrusterSystem {
    fn block_removed(&mut self, old_prop: &ThrusterProperty) {
        self.energy_consumption -= old_prop.energy_consupmtion;
        self.thrust_total -= old_prop.strength;
    }

    fn block_added(&mut self, prop: &ThrusterProperty) {
        self.energy_consumption += prop.energy_consupmtion;
        self.thrust_total += prop.strength;
    }
}

fn register_thruster_blocks(blocks: Res<Registry<Block>>, mut storage: ResMut<ThrusterBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:thruster") {
        storage.insert(
            block,
            ThrusterProperty {
                strength: 10.0,
                energy_consupmtion: 100.0,
            },
        );
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        storage.insert(
            block,
            ThrusterProperty {
                strength: 1.0,
                energy_consupmtion: 100.0,
            },
        )
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    energy_storage_blocks: Res<ThrusterBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut ThrusterSystem>,
    systems_query: Query<&Systems>,
) {
    for ev in event.iter() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                    system.block_removed(prop);
                }

                if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                    system.block_added(prop);
                }
            }
        }
    }
}

fn update_movement(
    thrusters_query: Query<(&ThrusterSystem, &StructureSystem)>,
    mut query: Query<
        (
            &ShipMovement,
            &Systems,
            &Transform,
            &mut Velocity,
            &mut ExternalImpulse,
            &ReadMassProperties,
        ),
        With<Pilot>,
    >,
    mut energy_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (thruster_system, system) in thrusters_query.iter() {
        if let Ok((movement, systems, transform, mut velocity, mut external_impulse, readmass)) = query.get_mut(system.structure_entity) {
            // Rotation
            let torque = Quat::from_affine3(&transform.compute_affine()).mul(movement.torque * 5.0);

            const MAX_ANGLE_PER_SECOND: f32 = 100.0;

            let max = MAX_ANGLE_PER_SECOND * time.delta_seconds();

            velocity.angvel = torque.clamp_length(0.0, max);

            velocity.linvel = velocity.linvel.clamp_length(0.0, MAX_SHIP_SPEED);

            // Position
            let normal = movement.into_normal_vector();

            let mut movement_vector = if normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0 {
                Vec3::ZERO
            } else {
                let mut movement_vector = transform.forward() * normal.z;
                movement_vector += transform.right() * normal.x;
                movement_vector += transform.up() * normal.y;

                movement_vector = movement_vector.normalize();

                let delta = time.delta_seconds();

                let mut energy_used = thruster_system.energy_consumption * delta;

                let ratio;

                if let Ok(mut energy_system) = systems.query_mut(&mut energy_query) {
                    if energy_used > energy_system.get_energy() {
                        ratio = energy_system.get_energy() / energy_used;
                        energy_used = energy_system.get_energy();
                    } else {
                        ratio = 1.0;
                    }

                    energy_system.decrease_energy(energy_used);

                    movement_vector * (thruster_system.thrust_total * ratio)
                } else {
                    Vec3::ZERO
                }
            };

            if movement.braking {
                let mut brake_vec = -velocity.linvel * readmass.0.mass;
                let delta = time.delta_seconds() * MAX_BRAKE_DELTA_PER_THRUST * thruster_system.thrust_total;

                if brake_vec.length_squared() >= delta * delta {
                    brake_vec = brake_vec.normalize() * delta;
                }

                movement_vector += brake_vec;
            }

            external_impulse.impulse += movement_vector;
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    thruster_blocks: Res<ThrusterBlocks>,
) {
    for ev in event_reader.iter() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = ThrusterSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = thruster_blocks.get(block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system);
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.insert_resource(ThrusterBlocks::default())
        .add_systems(OnEnter(post_loading_state), register_thruster_blocks)
        .add_systems(
            Update,
            (structure_loaded_event, block_update_system, update_movement).run_if(in_state(playing_state)),
        )
        .register_type::<ThrusterSystem>();
}
