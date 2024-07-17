//! Server-side laser cannon logic

use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier3d::{plugin::RapierContextEntityLink, prelude::Velocity};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    block::{block_events::BlockEventsSet, Block},
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerStructureSystemMessages, system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    physics::location::Location,
    projectiles::laser::Laser,
    registry::Registry,
    structure::{
        systems::{
            energy_storage_system::EnergyStorageSystem,
            laser_cannon_system::{LaserCannonCalculator, LaserCannonProperty, LaserCannonSystem, SystemCooldown},
            line_system::LineBlocks,
            StructureSystem, StructureSystems, StructureSystemsSet, SystemActive,
        },
        Structure,
    },
};

use crate::state::GameState;

use super::{line_system::add_line_system, sync::register_structure_system, thruster_system};

fn on_add_laser(mut commands: Commands, query: Query<Entity, Added<LaserCannonSystem>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(SystemCooldown {
            cooldown_time: Duration::from_millis(1000),
            ..Default::default()
        });
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
    mut query: Query<(&LaserCannonSystem, &StructureSystem, &mut SystemCooldown), With<SystemActive>>,
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
    for (cannon_system, system, mut cooldown) in query.iter_mut() {
        if let Ok((ship_entity, systems, structure, location, global_transform, ship_velocity, physics_world)) =
            systems.get(system.structure_entity())
        {
            if let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query) {
                let sec = time.elapsed_seconds();

                if sec - cooldown.last_use_time > cooldown.cooldown_time.as_secs_f32() {
                    cooldown.last_use_time = sec;

                    let mut any_fired = false;

                    for line in cannon_system.lines.iter() {
                        if energy_storage_system.get_energy() >= line.property.energy_per_shot {
                            any_fired = true;
                            energy_storage_system.decrease_energy(line.property.energy_per_shot);

                            let location = structure.block_world_location(line.start.coords(), global_transform, location);

                            let relative_direction = line.direction.direction_vec3();
                            let laser_velocity = global_transform.affine().matrix3.mul_vec3(relative_direction) * LASER_BASE_VELOCITY;

                            let strength = (5.0 * line.len as f32).powf(1.2);
                            let no_hit = Some(system.structure_entity());

                            Laser::spawn(
                                location,
                                laser_velocity,
                                ship_velocity.linvel,
                                strength,
                                no_hit,
                                &time,
                                *physics_world,
                                &mut commands,
                            );

                            let color = line.color;

                            server.broadcast_message(
                                NettyChannelServer::StructureSystems,
                                cosmos_encoder::serialize(&ServerStructureSystemMessages::CreateLaser {
                                    color,
                                    location,
                                    laser_velocity,
                                    firer_velocity: ship_velocity.linvel,
                                    strength,
                                    no_hit,
                                }),
                            );
                        } else {
                            break;
                        }
                    }

                    if any_fired {
                        server.broadcast_message(
                            NettyChannelServer::StructureSystems,
                            cosmos_encoder::serialize(&ServerStructureSystemMessages::LaserCannonSystemFired { ship_entity }),
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_line_system::<LaserCannonProperty, LaserCannonCalculator>(app);

    app.add_systems(
        Update,
        update_system
            .ambiguous_with(thruster_system::update_ship_force_and_velocity)
            .after(BlockEventsSet::ProcessEventsPostPlacement)
            .in_set(StructureSystemsSet::UpdateSystems)
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::PostLoading), register_laser_blocks)
    .add_systems(Update, on_add_laser.after(StructureSystemsSet::UpdateSystems));

    register_structure_system::<LaserCannonSystem>(app, true, "cosmos:laser_cannon");
}
