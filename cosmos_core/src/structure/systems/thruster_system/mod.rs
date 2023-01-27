use bevy::{
    ecs::schedule::StateData,
    prelude::{
        App, Commands, Component, CoreStage, EventReader, Query, Res, ResMut, Resource, SystemSet,
        Transform, Vec3, With,
    },
    reflect::{FromReflect, Reflect},
    time::Time,
    utils::HashMap,
};
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, Velocity};
use iyes_loopless::prelude::*;

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

#[derive(Component, Default, Reflect, FromReflect)]
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
                strength: 2.0,
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
        if let Ok(mut system) = systems_query
            .get(ev.structure_entity)
            .expect("Structure should have Systems component")
            .query_mut(&mut system_query)
        {
            if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                system.block_removed(prop);
            }

            if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(prop);
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
            &ReadMassProperties,
            &mut ExternalImpulse,
        ),
        With<Pilot>,
    >,
    mut energy_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (thruster_system, system) in thrusters_query.iter() {
        if let Ok((movement, systems, transform, mut velocity, mass_props, mut external_impulse)) =
            query.get_mut(system.structure_entity)
        {
            let normal = movement.into_normal_vector();
            // velocity.angvel += transform.rotation.mul_vec3(movement.torque.clone());
            velocity.angvel += transform.rotation.mul_vec3(movement.torque)
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

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_state: T,
) {
    app.insert_resource(ThrusterBlocks::default())
        .add_system_set(
            SystemSet::on_enter(post_loading_state).with_system(register_thruster_blocks),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(structure_loaded_event))
        .add_system_set(SystemSet::on_update(playing_state).with_system(update_movement))
        .register_type::<ThrusterSystem>();
}
