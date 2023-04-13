use bevy::{prelude::*, time::Time};
use bevy_rapier3d::prelude::{PhysicsWorld, Velocity, DEFAULT_WORLD_ID};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerLaserCannonSystemMessages,
        NettyChannel,
    },
    physics::{location::Location, player_world::PlayerWorld},
    projectiles::laser::Laser,
    structure::{
        systems::{
            energy_storage_system::EnergyStorageSystem, laser_cannon_system::LaserCannonSystem,
            StructureSystem, SystemActive, Systems,
        },
        Structure,
    },
};

use crate::state::GameState;

const LASER_BASE_VELOCITY: f32 = 200.0;
const LASER_SHOOT_SECONDS: f32 = 0.2;

fn update_system(
    mut query: Query<(&mut LaserCannonSystem, &StructureSystem), With<SystemActive>>,
    mut es_query: Query<&mut EnergyStorageSystem>,
    systems: Query<(
        &Systems,
        &Structure,
        &Location,
        &GlobalTransform,
        &Velocity,
        Option<&PhysicsWorld>,
    )>,
    world_query: Query<(&Location, &PhysicsWorld), With<PlayerWorld>>,
    time: Res<Time>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
) {
    for (mut cannon_system, system) in query.iter_mut() {
        if let Ok((systems, structure, location, global_transform, ship_velocity, physics_world)) =
            systems.get(system.structure_entity)
        {
            if let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query) {
                let sec = time.elapsed_seconds();

                if sec - cannon_system.last_shot_time > LASER_SHOOT_SECONDS {
                    cannon_system.last_shot_time = sec;

                    let world_id = physics_world
                        .map(|bw| bw.world_id)
                        .unwrap_or(DEFAULT_WORLD_ID);

                    if let Some(world_location) = world_query
                        .iter()
                        .find(|(_, pw)| pw.world_id == world_id)
                        .map(|(loc, _)| loc)
                    {
                        for line in cannon_system.lines.iter() {
                            if energy_storage_system.get_energy() >= line.property.energy_per_shot {
                                energy_storage_system
                                    .decrease_energy(line.property.energy_per_shot);

                                let location = structure.block_world_location(
                                    line.start.x,
                                    line.start.y,
                                    line.start.z,
                                    global_transform,
                                    location,
                                );

                                // AT SOME POINT, THE NEGATIVE SIGN HAS TO BE REMOVED HERE!!!!!
                                // I SHOULD NOT HAVE TO NEGATE THE DIRECTION
                                // SINCE THERE IS NO WAY TO ROTATE THE CANNONS, FOR NOW THIS HAS
                                // TO BE HERE, BUT ONCE CANNONS CAN BE ROTATED, REMOVE THIS!
                                let laser_velocity = global_transform
                                    .affine()
                                    .matrix3
                                    .mul_vec3(-line.direction.direction_vec3())
                                    * LASER_BASE_VELOCITY;

                                let strength = (5.0 * line.len as f32).powf(1.2);
                                let no_hit = Some(system.structure_entity);

                                Laser::spawn(
                                    location,
                                    laser_velocity,
                                    ship_velocity.linvel,
                                    strength,
                                    no_hit,
                                    &time,
                                    world_id,
                                    world_location,
                                    &mut commands,
                                );

                                let color =
                                    Color::rgb(rand::random(), rand::random(), rand::random());

                                server.broadcast_message(
                                    NettyChannel::LaserCannonSystem.id(),
                                    cosmos_encoder::serialize(
                                        &ServerLaserCannonSystemMessages::CreateLaser {
                                            color,
                                            location,
                                            laser_velocity,
                                            firer_velocity: ship_velocity.linvel,
                                            strength,
                                            no_hit,
                                        },
                                    ),
                                );
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(update_system.in_set(OnUpdate(GameState::Playing)));
}
