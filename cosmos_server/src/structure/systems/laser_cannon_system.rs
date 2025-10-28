//! Server-side laser cannon logic

use std::time::Duration;

use crate::logic::{LogicInputEvent, LogicSystemSet, logic_driver::LogicDriver};
use bevy::prelude::*;
use bevy_rapier3d::{plugin::RapierContextEntityLink, prelude::Velocity};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    netty::{
        NettyChannelServer, cosmos_encoder,
        server_laser_cannon_system_messages::{LaserLoc, ServerStructureSystemMessages},
    },
    physics::location::{Location, LocationPhysicsSet},
    projectiles::{causer::Causer, laser::Laser},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        systems::{
            StructureSystem, StructureSystems, StructureSystemsSet, SystemActive,
            energy_storage_system::EnergyStorageSystem,
            laser_cannon_system::{LaserCannonCalculator, LaserCannonProperty, LaserCannonSystem, LineSystemCooldown, SystemCooldown},
            line_system::LineBlocks,
        },
    },
};

use super::{line_system::add_line_system, sync::register_structure_system, thruster_system};

fn on_add_laser(mut commands: Commands, query: Query<Entity, Added<LaserCannonSystem>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(LineSystemCooldown::default());
    }
}

fn register_laser_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<LaserCannonProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:laser_cannon") {
        cannon.insert(block, LaserCannonProperty { energy_per_shot: 100.0 })
    }
}

/// How fast a laser will travel (m/s) ignoring the speed of its shooter.
pub const LASER_BASE_VELOCITY: f32 = 200.0;

fn update_system(
    mut query: Query<(&LaserCannonSystem, &StructureSystem, &mut LineSystemCooldown, Has<SystemActive>)>,
    mut es_query: Query<&mut EnergyStorageSystem>,
    systems: Query<(
        Entity,
        &StructureSystems,
        &Structure,
        &Location,
        &GlobalTransform,
        &Velocity,
        &RapierContextEntityLink,
    )>,
    time: Res<Time>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
) {
    for (cannon_system, system, mut cooldown, system_active) in query.iter_mut() {
        let Ok((ship_entity, systems, structure, structure_location, global_transform, ship_velocity, physics_world)) =
            systems.get(system.structure_entity())
        else {
            continue;
        };
        let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query) else {
            continue;
        };

        let sec = time.elapsed_secs();

        let mut any_fired = false;

        let default_cooldown = SystemCooldown {
            cooldown_time: Duration::from_millis(1000),
            ..Default::default()
        };

        cooldown.remove_unused_cooldowns(cannon_system);

        for line in cannon_system.lines.iter() {
            let cooldown = cooldown.lines.entry(line.start).or_insert(default_cooldown);

            if sec - cooldown.last_use_time < cooldown.cooldown_time.as_secs_f32() {
                continue;
            }

            if !((system_active || line.active()) && energy_storage_system.get_energy() >= line.property.energy_per_shot) {
                continue;
            }

            cooldown.last_use_time = sec;
            any_fired = true;
            energy_storage_system.decrease_energy(line.property.energy_per_shot);

            let loc = LaserLoc::Relative {
                entity: system.structure_entity(),
                offset: structure.block_relative_position(line.end()),
            };

            let relative_direction = line.direction.as_vec3();
            let laser_velocity = global_transform.affine().matrix3.mul_vec3(relative_direction) * LASER_BASE_VELOCITY;

            let strength = (5.0 * line.len as f32).powf(1.2);
            let no_hit = Some(system.structure_entity());

            let causer = Some(Causer(system.structure_entity()));

            Laser::spawn(
                loc,
                laser_velocity,
                ship_velocity.linvel,
                strength,
                no_hit,
                &time,
                *physics_world,
                &mut commands,
                causer,
            );

            let color = line.color;

            server.broadcast_message(
                NettyChannelServer::StructureSystems,
                cosmos_encoder::serialize(&ServerStructureSystemMessages::CreateLaser {
                    color,
                    location: loc,
                    laser_velocity,
                    firer_velocity: ship_velocity.linvel,
                    strength,
                    no_hit,
                    causer,
                }),
            );
        }

        if any_fired {
            server.broadcast_message(
                NettyChannelServer::StructureSystems,
                cosmos_encoder::serialize(&ServerStructureSystemMessages::LaserCannonSystemFired { ship_entity }),
            );
        }
    }
}

fn laser_cannon_input_event_listener(
    mut evr_logic_input: EventReader<LogicInputEvent>,
    blocks: Res<Registry<Block>>,
    mut q_logic_driver: Query<&mut LogicDriver>,
    q_structure: Query<(&Structure, &StructureSystems)>,
    mut q_laser_cannon_system: Query<&mut LaserCannonSystem>,
) {
    for ev in evr_logic_input.read() {
        let Ok((structure, systems)) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if structure.block_at(ev.block.coords(), &blocks).unlocalized_name() != "cosmos:laser_cannon" {
            continue;
        }
        let Ok(logic_driver) = q_logic_driver.get_mut(ev.block.structure()) else {
            continue;
        };
        let Ok(mut laser_cannon_system) = systems.query_mut(&mut q_laser_cannon_system) else {
            continue;
        };
        let Some(line) = laser_cannon_system.mut_line_containing(ev.block.coords()) else {
            continue;
        };

        let active = logic_driver
            .read_all_inputs(ev.block.coords(), structure.block_rotation(ev.block.coords()))
            .iter()
            .any(|signal| *signal != 0);

        if active {
            line.mark_block_active(ev.block.coords());
        } else {
            line.mark_block_inactive(ev.block.coords());
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_line_system::<LaserCannonProperty, LaserCannonCalculator>(app);

    app.add_systems(
        FixedUpdate,
        update_system
            .ambiguous_with(thruster_system::update_ship_force_and_velocity)
            // .after(BlockEventsSet::ProcessEvents)
            .in_set(StructureSystemsSet::UpdateSystemsBlocks)
            .before(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::PostLoading), register_laser_blocks)
    .add_systems(
        FixedUpdate,
        on_add_laser
            .before(laser_cannon_input_event_listener)
            .after(StructureSystemsSet::UpdateSystemsBlocks),
    )
    .add_systems(
        FixedUpdate,
        laser_cannon_input_event_listener
            .in_set(StructureSystemsSet::UpdateSystems)
            .in_set(LogicSystemSet::Consume)
            .ambiguous_with(LogicSystemSet::Consume),
    );

    register_structure_system::<LaserCannonSystem>(app, true, "cosmos:laser_cannon");
}
