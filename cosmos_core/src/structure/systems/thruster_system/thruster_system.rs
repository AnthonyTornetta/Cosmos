use bevy::{
    ecs::schedule::StateData,
    prelude::{
        App, Commands, Component, CoreStage, Entity, EventReader, Query, Res, ResMut, Resource,
        SystemSet, Transform, Vec3, With,
    },
    time::Time,
    utils::HashMap,
};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, Velocity};
use iyes_loopless::prelude::*;

use crate::{
    block::{blocks::Blocks, Block},
    events::block_events::BlockChangedEvent,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        events::ChunkSetEvent,
        ship::{pilot::Pilot, ship_movement::ShipMovement},
        structure::Structure,
        systems::energy_storage_system::energy_storage_system::EnergyStorageSystem,
    },
};

pub struct ThrusterProperty {
    pub strength: f32,
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

#[derive(Component, Default, Inspectable)]
pub struct ThrusterSystem {
    thrust_total: f32,
    energy_consumption: f32,
}

fn register_thruster_blocks(blocks: Res<Blocks>, mut storage: ResMut<ThrusterBlocks>) {
    if let Some(block) = blocks.block_from_id("cosmos:thruster") {
        storage.insert(
            block,
            ThrusterProperty {
                strength: 2.0,
                energy_consupmtion: 100.0,
            },
        );
    }

    if let Some(block) = blocks.block_from_id("cosmos:ship_core") {
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
    mut commands: Commands,
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    energy_storage_blocks: Res<ThrusterBlocks>,
    blocks: Res<Blocks>,
    mut system_query: Query<&mut ThrusterSystem>,
    structure_query: Query<&Structure>,
) {
    for ev in event.iter() {
        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            if let Some(es) = energy_storage_blocks.get(blocks.block_from_numeric_id(ev.old_block))
            {
                system.energy_consumption -= es.energy_consupmtion;
                system.thrust_total -= es.strength;
            }

            if let Some(es) = energy_storage_blocks.get(blocks.block_from_numeric_id(ev.new_block))
            {
                system.energy_consumption += es.energy_consupmtion;
                system.thrust_total += es.strength;
            }
        } else {
            let mut system = ThrusterSystem::default();

            if let Some(es) = energy_storage_blocks.get(blocks.block_from_numeric_id(ev.old_block))
            {
                system.energy_consumption -= es.energy_consupmtion;
                system.thrust_total -= es.strength;
            }

            if let Some(es) = energy_storage_blocks.get(blocks.block_from_numeric_id(ev.new_block))
            {
                system.energy_consumption += es.energy_consupmtion;
                system.thrust_total += es.strength;
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }

    // ChunkSetEvents should not overwrite existing blocks, so no need to check for that
    for ev in chunk_set_event.iter() {
        let structure = structure_query.get(ev.structure_entity).unwrap();

        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
                for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                    for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                        let b = structure.block_at(x, y, z);

                        if energy_storage_blocks.blocks.contains_key(&b) {
                            let prop = energy_storage_blocks
                                .get(blocks.block_from_numeric_id(b))
                                .unwrap();

                            system.thrust_total += prop.strength;
                            system.energy_consumption += prop.energy_consupmtion;
                        }
                    }
                }
            }
        } else {
            let mut system = ThrusterSystem::default();

            for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
                for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                    for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                        let b = structure.block_at(x, y, z);

                        if energy_storage_blocks.blocks.contains_key(&b) {
                            let prop = energy_storage_blocks
                                .get(blocks.block_from_numeric_id(b))
                                .unwrap();

                            system.thrust_total += prop.strength;
                            system.energy_consumption += prop.energy_consupmtion;
                        }
                    }
                }
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }
}

fn update_movement(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &ShipMovement,
            &ThrusterSystem,
            &mut EnergyStorageSystem,
            &Transform,
            &mut Velocity,
            &ReadMassProperties,
        ),
        With<Pilot>,
    >,
    time: Res<Time>,
) {
    for (
        entity,
        movement,
        thruster_system,
        mut energy_system,
        transform,
        mut velocity,
        mass_props,
    ) in query.iter_mut()
    {
        let normal = movement.into_normal_vector();
        // velocity.angvel += transform.rotation.mul_vec3(movement.torque.clone());
        velocity.angvel += transform.rotation.mul_vec3(movement.torque.clone())
            / mass_props.0.principal_inertia
            * thruster_system.thrust_total;
        // This is horrible, please find something better
        velocity.angvel = velocity
            .angvel
            .clamp_length(0.0, movement.torque.length() * 4.0);

        velocity.linvel = velocity.linvel.clamp_length(0.0, 256.0);

        let movement_vector = if normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0 {
            Vec3::ZERO
        } else {
            let mut movement_vector = transform.forward() * normal.z;
            movement_vector += transform.right() * normal.x;
            movement_vector += transform.up() * normal.y;

            movement_vector = movement_vector.normalize();

            let delta = time.delta_seconds();

            let mut energy_used = thruster_system.energy_consumption * delta;

            let ratio;
            if energy_used > energy_system.get_energy() {
                ratio = energy_system.get_energy() / energy_used;
                energy_used = energy_system.get_energy();
            } else {
                ratio = 1.0;
            }

            energy_system.decrease_energy(energy_used);

            movement_vector * (thruster_system.thrust_total * ratio)
        };

        commands.entity(entity).insert(ExternalImpulse {
            impulse: movement_vector,
            // torque_impulse: movement.torque,
            ..Default::default()
        });
    }
}

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.insert_resource(ThrusterBlocks::default())
        .add_system_set(
            SystemSet::on_enter(post_loading_state).with_system(register_thruster_blocks),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state.clone()),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(update_movement))
        .register_inspectable::<ThrusterSystem>();
}
