//! The thrusters that move a ship

use bevy::prelude::*;
use bevy_rapier3d::{
    plugin::PhysicsSet,
    prelude::{ExternalImpulse, ReadMassProperties, Velocity},
};
use cosmos_core::{
    block::{Block, block_events::BlockEventsSet},
    events::block_events::BlockChangedEvent,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::Location,
    prelude::FullStructure,
    registry::Registry,
    state::GameState,
    structure::{
        Structure, StructureTypeSet,
        events::StructureLoadedEvent,
        ship::{
            Ship,
            pilot::{Pilot, PilotFocused},
            ship_movement::{ShipMovement, ShipMovementSet},
        },
        systems::{
            StructureSystem, StructureSystemType, StructureSystems, StructureSystemsSet,
            dock_system::Docked,
            energy_storage_system::EnergyStorageSystem,
            thruster_system::{ThrusterBlocks, ThrusterProperty, ThrusterSystem},
        },
    },
};

use super::sync::register_structure_system;

const MAX_SHIP_SPEED: f32 = 350.0;
const MAX_BRAKE_DELTA_PER_THRUST: f32 = 300.0;
const MAX_MATCH_SPEED_PER_THRUST: f32 = 10.0;

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
    systems_query: Query<&StructureSystems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.block.structure())
            && let Ok(mut system) = systems.query_mut(&mut system_query)
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

#[derive(Debug, Component, Reflect)]
/// A multiplier that changes the maximum speed of a ship. 1.0 is the default.
pub struct MaxShipSpeedModifier(pub f32);

pub(super) fn update_ship_force_and_velocity(
    thrusters_query: Query<(&ThrusterSystem, &StructureSystem)>,
    mut query: Query<
        (
            &Location,
            &mut Structure,
            &ShipMovement,
            &StructureSystems,
            &Transform,
            &mut ExternalImpulse,
            &ReadMassProperties,
            Option<&Docked>,
            Option<&MaxShipSpeedModifier>,
            Option<&PilotFocused>,
        ),
        (With<Pilot>, With<Ship>),
    >,
    q_loc: Query<&Location>,
    mut q_vel: Query<&mut Velocity>,
    mut energy_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (thruster_system, system) in thrusters_query.iter() {
        if let Ok((
            loc,
            mut structure,
            movement,
            systems,
            transform,
            mut external_impulse,
            readmass,
            docked,
            max_ship_speed_modifier,
            pilot_focused,
        )) = query.get_mut(system.structure_entity())
        {
            // Rotation
            if docked.is_none() {
                let Ok(mut velocity) = q_vel.get_mut(system.structure_entity()) else {
                    continue;
                };

                let torque = movement.torque * 5.0;

                const MAX_ANGLE_PER_SECOND: f32 = 500.0;
                const INVERSE_SCALING: f32 = 0.4;
                const THRUST_TORQUE_SCALING: f32 = 0.5;

                let bounds = FullStructure::placed_block_bounds(&mut structure);

                let max = bounds
                    .map(|(min, max)| {
                        // Need to add one because min and max equal each other if there is only 1
                        // block.

                        let d = Vec3::new((max.x - min.x) as f32, (max.y - min.y) as f32, (max.z - min.z) as f32);

                        // When rotating in one axis, your torque is based off the other 2 axis
                        (INVERSE_SCALING / (Vec3::ONE + Vec3::new(d.y + d.z, d.x + d.z, d.x + d.y) / 2.0))
                            * MAX_ANGLE_PER_SECOND
                            * (THRUST_TORQUE_SCALING * thruster_system.thrust_total().max(2.0).log2())
                            * time.delta_secs()
                    })
                    .unwrap_or(Vec3::ZERO);

                velocity.angvel = transform.rotation * torque.min(max).max(-max);

                let max_speed = MAX_SHIP_SPEED * max_ship_speed_modifier.map(|x| x.0).unwrap_or(1.0);
                velocity.linvel = velocity.linvel.clamp_length(0.0, max_speed);
            }

            // Position
            let normal = movement.into_normal_vector();

            let mut movement_vector = if normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0 {
                Vec3::ZERO
            } else {
                let mut movement_vector = transform.forward() * normal.z;
                movement_vector += transform.right() * normal.x;
                movement_vector += transform.up() * normal.y;

                movement_vector = movement_vector.normalize();

                let delta = time.delta_secs();

                let mut energy_used = thruster_system.energy_consumption() * delta;

                let ratio;

                if let Ok(mut energy_system) = systems.query_mut(&mut energy_query) {
                    if energy_used > energy_system.get_energy() {
                        ratio = energy_system.get_energy() / energy_used;
                        energy_used = energy_system.get_energy();
                    } else {
                        ratio = 1.0;
                    }

                    energy_system.decrease_energy(energy_used);

                    movement_vector * (thruster_system.thrust_total() * ratio)
                } else {
                    Vec3::ZERO
                }
            };

            if movement.match_speed
                && let Some(pilot_focused) = pilot_focused
                && let Ok(focused_loc) = q_loc.get(pilot_focused.0)
            {
                let diff = *focused_loc - *loc;
                let diff = diff.absolute_coords_f32();
                const MAX_FOCUS_DISTANCE: f32 = 2_000.0;
                if diff.length_squared() <= MAX_FOCUS_DISTANCE * MAX_FOCUS_DISTANCE {
                    let Ok(velocity) = q_vel.get(system.structure_entity()) else {
                        continue;
                    };

                    let other_vel = q_vel.get(pilot_focused.0).copied().unwrap_or_default();

                    let diff = other_vel.linvel - velocity.linvel;
                    let mut match_vec = diff.normalize_or_zero() * readmass.get().mass;
                    let delta = time.delta_secs() * MAX_MATCH_SPEED_PER_THRUST * thruster_system.thrust_total();

                    if match_vec.length_squared() >= delta * delta {
                        match_vec = match_vec.normalize() * delta;
                    }

                    movement_vector += match_vec;
                }
            }

            if movement.braking {
                let Ok(velocity) = q_vel.get(system.structure_entity()) else {
                    continue;
                };

                let mut brake_vec = -velocity.linvel * readmass.get().mass;
                let delta = time.delta_secs() * MAX_BRAKE_DELTA_PER_THRUST * thruster_system.thrust_total();

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
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    thruster_blocks: Res<ThrusterBlocks>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = ThrusterSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = thruster_blocks.get(structure.block_at(block, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Thrusters will run logic in these systems
pub enum ThrusterSystemSet {
    /// Thrust is applied as an external impulse to the ship, and power is consumed.
    ApplyThrusters,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(FixedUpdate, ThrusterSystemSet::ApplyThrusters);

    app.insert_resource(ThrusterBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_thruster_blocks)
        .add_systems(
            Update,
            (
                structure_loaded_event
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                block_update_system
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            )
                .chain()
                .in_set(NetworkingSystemsSet::Between)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            update_ship_force_and_velocity
                .before(PhysicsSet::SyncBackend)
                .after(ShipMovementSet::RemoveShipMovement)
                .in_set(ThrusterSystemSet::ApplyThrusters)
                .in_set(StructureSystemsSet::UpdateSystems)
                .in_set(StructureTypeSet::Ship),
        )
        .register_type::<ThrusterSystem>()
        .register_type::<MaxShipSpeedModifier>();

    register_structure_system::<ThrusterSystem>(app, false, "cosmos:thruster");
}
