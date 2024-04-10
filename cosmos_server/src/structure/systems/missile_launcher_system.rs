//! Server-side laser cannon logic

use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{PhysicsWorld, Velocity, DEFAULT_WORLD_ID};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    netty::{cosmos_encoder, server_laser_cannon_system_messages::ServerLaserCannonSystemMessages, NettyChannelServer},
    physics::location::Location,
    projectiles::missile::Missile,
    registry::Registry,
    structure::{
        systems::{
            energy_storage_system::EnergyStorageSystem,
            laser_cannon_system::SystemCooldown,
            line_system::LineBlocks,
            missile_launcher_system::{MissileLauncherCalculator, MissileLauncherProperty, MissileLauncherSystem},
            StructureSystem, SystemActive, Systems,
        },
        Structure,
    },
};

use crate::state::GameState;

use super::{line_system::add_line_system, sync::register_structure_system};

fn on_add_missile_launcher(mut commands: Commands, query: Query<Entity, Added<MissileLauncherSystem>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(SystemCooldown {
            cooldown_time: Duration::from_millis(1000),
            ..Default::default()
        });
    }
}

fn register_missile_launcher_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<MissileLauncherProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:missile_launcher") {
        cannon.insert(block, MissileLauncherProperty { energy_per_shot: 100.0 })
    }
}

/// How fast a laser will travel (m/s) ignoring the speed of its shooter.
pub const MISSILE_BASE_VELOCITY: f32 = 20.0;

const MISSILE_SPEED_MULTIPLIER: f32 = 30.0; // higher = higher speed for way less cannons
const MISSILE_SPEED_DIVIDER: f32 = 1.0 / 5.0; // lower = more cannons required for same effect

fn update_system(
    mut query: Query<(&MissileLauncherSystem, &StructureSystem, &mut SystemCooldown), With<SystemActive>>,
    mut es_query: Query<&mut EnergyStorageSystem>,
    systems: Query<(
        Entity,
        &Systems,
        &Structure,
        &Location,
        &GlobalTransform,
        &Velocity,
        Option<&PhysicsWorld>,
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

                    let world_id = physics_world.map(|bw| bw.world_id).unwrap_or(DEFAULT_WORLD_ID);

                    let mut any_fired = false;

                    for line in cannon_system.lines.iter() {
                        if energy_storage_system.get_energy() >= line.property.energy_per_shot {
                            any_fired = true;
                            energy_storage_system.decrease_energy(line.property.energy_per_shot);

                            let location = structure.block_world_location(line.start.coords(), global_transform, location);

                            let relative_direction = line.direction.direction_vec3();

                            let missile_vel =
                                MISSILE_BASE_VELOCITY + (line.len as f32 * MISSILE_SPEED_DIVIDER + 1.0).ln() * MISSILE_SPEED_MULTIPLIER;

                            let missile_velocity = global_transform.affine().matrix3.mul_vec3(relative_direction) * missile_vel;

                            let strength = (5.0 * line.len as f32).powf(1.2);
                            let no_hit = Some(system.structure_entity());

                            Missile::spawn(
                                location,
                                missile_velocity,
                                ship_velocity.linvel,
                                strength,
                                no_hit,
                                &time,
                                world_id,
                                &mut commands,
                            );

                            let color = line.color;

                            server.broadcast_message(
                                NettyChannelServer::LaserCannonSystem,
                                cosmos_encoder::serialize(&ServerLaserCannonSystemMessages::CreateMissile {
                                    color,
                                    location,
                                    laser_velocity: missile_velocity,
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
                            NettyChannelServer::LaserCannonSystem,
                            cosmos_encoder::serialize(&ServerLaserCannonSystemMessages::MissileLauncherSystemFired { ship_entity }),
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_line_system::<MissileLauncherProperty, MissileLauncherCalculator>(app);

    app.add_systems(Update, update_system.run_if(in_state(GameState::Playing)))
        .add_systems(OnEnter(GameState::PostLoading), register_missile_launcher_blocks)
        .add_systems(Update, on_add_missile_launcher);

    register_structure_system::<MissileLauncherSystem>(app, true, "cosmos:missile_launcher");
}
