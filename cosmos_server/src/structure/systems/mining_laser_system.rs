use std::time::Duration;

use bevy::{prelude::*, time::Time, utils::HashMap};
use bevy_rapier3d::{
    pipeline::QueryFilter,
    plugin::RapierContext,
    prelude::{PhysicsWorld, Velocity, DEFAULT_WORLD_ID},
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        block_events::{BlockBreakEvent, BlockEventsSet},
        Block,
    },
    ecs::NeedsDespawned,
    netty::{cosmos_encoder, server_laser_cannon_system_messages::ServerLaserCannonSystemMessages, NettyChannelServer},
    physics::location::Location,
    projectiles::laser::Laser,
    registry::Registry,
    structure::{
        coordinates::BlockCoordinate,
        structure_block::StructureBlock,
        systems::{
            energy_storage_system::EnergyStorageSystem,
            laser_cannon_system::{LaserCannonSystem, SystemCooldown},
            mining_laser_system::{MiningLaserProperty, MiningLaserSystem},
            StructureSystem, SystemActive, Systems,
        },
        Structure,
    },
};

use crate::state::GameState;

const LASER_BASE_VELOCITY: f32 = 200.0;

// struct MiningBeam {
//     direction: Vec3,
//     last_mined: Option<(StructureBlock, f32)>,
//     mine_duration: Duration,
//     property: MiningLaserProperty,
// }

// #[derive(Deref, DerefMut, Default, Component)]
// struct MiningBeams(HashMap<BlockCoordinate, MiningBeam>);

// fn on_add_system(mut commands: Commands, query: Query<Entity, Added<MiningLaserSystem>>) {
//     for ent in query.iter() {
//         commands.entity(ent).insert(MiningBeams::default());
//     }
// }

const BEAM_MAX_RANGE: f32 = 250.0;
const BREAK_DECAY_RATIO: f32 = 0.5;

#[derive(Component)]
struct MiningBlock {
    block_coord: BlockCoordinate,
    time_mined: f32,
    dirty: bool,
    last_toucher: Entity,
}

#[derive(Component, Default)]
struct BeingMined(HashMap<BlockCoordinate, Entity>);

impl BeingMined {}

fn add_being_mined(mut commands: Commands, query: Query<Entity, (With<Structure>, Without<BeingMined>)>) {
    for ent in query.iter() {
        commands.entity(ent).insert(BeingMined::default());
    }
}

fn check_should_break(
    mut commands: Commands,
    mut q_structure: Query<(Entity, &mut Structure, &mut BeingMined)>,
    mut q_mining_blocks: Query<&mut MiningBlock>,
    mut ev_writer: EventWriter<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    time: Res<Time>,
) {
    let delta = time.delta_seconds();

    for (structure_entity, mut structure, mut being_mined) in q_structure.iter_mut() {
        being_mined.0.retain(|coordinate, &mut entity| {
            let Ok(mut mining_block) = q_mining_blocks.get_mut(entity) else {
                return false;
            };

            let block = structure.block_at(mining_block.block_coord, &blocks);

            println!("Mining: {block:?}");

            if mining_block.time_mined >= 3.0 {
                ev_writer.send(BlockBreakEvent {
                    block: StructureBlock::new(*coordinate),
                    breaker: mining_block.last_toucher,
                    structure_entity,
                });
                commands.entity(entity).insert(NeedsDespawned);
                return false;
            }

            if !mining_block.dirty {
                mining_block.time_mined -= delta * BREAK_DECAY_RATIO;
            }

            if mining_block.time_mined <= 0.0 {
                commands.entity(entity).insert(NeedsDespawned);
                return false;
            }

            true
        });
    }
}

fn update_mining_beams(
    mut commands: Commands,
    mut q_mining_beams: Query<(Entity, &mut MiningBeam, &PhysicsWorld, &GlobalTransform)>,
    mut q_structure: Query<(Entity, &mut Structure, &mut BeingMined, &GlobalTransform)>,
    mut q_mining_block: Query<&mut MiningBlock>,
    q_is_system_active: Query<(), With<SystemActive>>,
    rapier_context: Res<RapierContext>,
    q_parent: Query<&Parent>,
    time: Res<Time>,
) {
    for (entity, beam, p_world, g_trans) in q_mining_beams.iter_mut() {
        if !q_is_system_active.contains(beam.system_entity) {
            commands.entity(entity).insert(NeedsDespawned);
            continue;
        }

        let ray_start = g_trans.translation();
        let ray_dir = g_trans.forward();

        if let Ok(Some((hit_entity, toi))) = rapier_context.cast_ray(
            p_world.world_id,
            ray_start.into(),
            ray_dir.into(),
            BEAM_MAX_RANGE,
            true,
            QueryFilter::predicate(QueryFilter::default(), &|entity| {
                if beam.structure_entity == entity {
                    false
                } else if let Ok(parent) = q_parent.get(entity) {
                    parent.get() != beam.structure_entity
                } else {
                    false
                }
            }),
        ) {
            println!("Hit!!!");
            commands.entity(hit_entity).log_components();

            let mut handle_structure = |structure_entity: Entity,
                                        structure: &mut Structure,
                                        being_mined: &mut BeingMined,
                                        structure_global_trans: &GlobalTransform| {
                let global_point_hit = ray_start + (ray_dir * (toi + 0.01));

                let local_point_hit = Quat::from_affine3(&structure_global_trans.affine())
                    .inverse()
                    .mul_vec3(global_point_hit - structure_global_trans.translation());

                println!("Hit {global_point_hit} => {local_point_hit}");

                if let Ok(block_coord) =
                    structure.relative_coords_to_local_coords_checked(local_point_hit.x, local_point_hit.y, local_point_hit.z)
                {
                    let delta_time = time.delta_seconds();

                    if let Some(&mining_block) = being_mined.0.get(&block_coord) {
                        if let Ok(mut mining_block) = q_mining_block.get_mut(mining_block) {
                            mining_block.time_mined += delta_time;
                            mining_block.dirty = true;
                        }
                    } else {
                        let mining_block = commands
                            .spawn((
                                Name::new("Block being mined"),
                                MiningBlock {
                                    block_coord,
                                    time_mined: delta_time,
                                    dirty: true,
                                    last_toucher: structure_entity,
                                },
                            ))
                            .id();

                        being_mined.0.insert(block_coord, mining_block);
                    }
                } else {
                    println!("Oob hit??");
                }
                // if let Some((block, seconds_mined)) = &mut beam.mining {
                //     if *block != local_point_hit {}
                // }
            };

            if let Ok((entity, mut structure, mut being_mined, g_trans)) = q_structure.get_mut(hit_entity) {
                handle_structure(entity, &mut structure, &mut being_mined, g_trans);
            } else if let Ok(parent) = q_parent.get(hit_entity) {
                if let Ok((entity, mut structure, mut being_mined, g_trans)) = q_structure.get_mut(parent.get()) {
                    handle_structure(entity, &mut structure, &mut being_mined, g_trans);
                }
            }
        } else {
            println!("Ray missed!");
        }
    }
}

#[derive(Component)]
struct MiningBeam {
    mining: Option<(StructureBlock, f32)>,
    mine_duration: Duration,
    property: MiningLaserProperty,
    system_entity: Entity,
    structure_entity: Entity,
}

fn on_activate_system(
    mut query: Query<(Entity, &MiningLaserSystem, &StructureSystem), Added<SystemActive>>,
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
    for (system_entity, mining_system, system) in query.iter_mut() {
        if let Ok((ship_entity, systems, structure, location, global_transform, ship_velocity, physics_world)) =
            systems.get(system.structure_entity)
        {
            if let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query) {
                let sec = time.elapsed_seconds();

                let world_id = physics_world.map(|bw| bw.world_id).unwrap_or_default();

                for line in mining_system.lines.iter() {
                    let energy = line.property.energy_per_second * sec;

                    if energy_storage_system.decrease_energy(energy) {
                        // AT SOME POINT, THE NEGATIVE SIGN HAS TO BE REMOVED HERE!!!!!
                        // I SHOULD NOT HAVE TO NEGATE THE DIRECTION
                        // SINCE THERE IS NO WAY TO ROTATE THE CANNONS, FOR NOW THIS HAS
                        // TO BE HERE, BUT ONCE CANNONS CAN BE ROTATED, REMOVE THIS!
                        let beam_direction = global_transform.affine().matrix3.mul_vec3(-line.direction.direction_vec3());

                        let strength = line.property.break_speed;

                        let beam_begin = line.end();
                        let rel_pos = structure.block_relative_position(beam_begin.coords());

                        let mining_beam = commands
                            .spawn((
                                Name::new("Mining beam"),
                                MiningBeam {
                                    mine_duration: strength,
                                    mining: None,
                                    property: line.property,
                                    structure_entity: ship_entity,
                                    system_entity,
                                },
                                TransformBundle::from_transform(Transform::from_translation(rel_pos).looking_to(beam_direction, Vec3::Y)),
                                PhysicsWorld { world_id },
                            ))
                            .id();

                        commands.entity(ship_entity).add_child(mining_beam);
                    } else {
                        // Not enough power for all the beams, don't bother turning them on for a single frame.
                        break;
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (add_being_mined, on_activate_system, update_mining_beams, check_should_break)
            .before(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    );
}
