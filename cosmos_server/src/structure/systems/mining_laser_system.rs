use bevy::{prelude::*, time::Time, utils::HashMap};
use bevy_rapier3d::{
    pipeline::QueryFilter,
    plugin::RapierContext,
    prelude::{PhysicsWorld, Velocity},
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        block_events::{BlockBreakEvent, BlockEventsSet},
        Block,
    },
    ecs::NeedsDespawned,
    physics::location::Location,
    registry::Registry,
    structure::{
        coordinates::BlockCoordinate,
        structure_block::StructureBlock,
        systems::{
            energy_storage_system::EnergyStorageSystem,
            mining_laser_system::{MiningLaserProperty, MiningLaserSystem},
            StructureSystem, SystemActive, Systems,
        },
        Structure,
    },
};

use crate::state::GameState;

const BEAM_MAX_RANGE: f32 = 250.0;
const BREAK_DECAY_RATIO: f32 = 1.5;

#[derive(Component, Debug)]
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
    mut q_structure: Query<(Entity, &Structure, &mut BeingMined)>,
    mut q_mining_blocks: Query<&mut MiningBlock>,
    mut ev_writer: EventWriter<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    time: Res<Time>,
) {
    let delta = time.delta_seconds();

    for (structure_entity, structure, mut being_mined) in q_structure.iter_mut() {
        being_mined.0.retain(|coordinate, &mut entity| {
            let Ok(mut mining_block) = q_mining_blocks.get_mut(entity) else {
                return false;
            };

            let block = structure.block_at(mining_block.block_coord, &blocks);

            println!("Mining: {block:?} {mining_block:?}");

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

            mining_block.dirty = false;

            true
        });
    }
}

fn update_mining_beams(
    mut commands: Commands,
    mut q_mining_beams: Query<(Entity, &mut MiningBeam, &PhysicsWorld, &GlobalTransform)>,
    q_systems: Query<&Systems>,
    mut q_energy_storage_system: Query<&mut EnergyStorageSystem>,
    q_structure: Query<(&Structure, &GlobalTransform)>,
    mut q_mining_block: Query<&mut MiningBlock>,
    mut q_being_mined: Query<&mut BeingMined>,
    q_is_system_active: Query<(), With<SystemActive>>,
    rapier_context: Res<RapierContext>,
    q_parent: Query<&Parent>,
    time: Res<Time>,
) {
    #[derive(Debug)]
    struct CachedBlockBeingMined {
        hit_structure_entity: Entity,
        beam_shooter_entity: Entity,
        hit_coordinate: BlockCoordinate,
        break_increase: f32,
    }

    let mut mining_blocks: Vec<CachedBlockBeingMined> = vec![];

    let delta_time = time.delta_seconds();

    for (entity, beam, p_world, g_trans) in q_mining_beams.iter_mut() {
        if !q_is_system_active.contains(beam.system_entity) {
            commands.entity(entity).insert(NeedsDespawned);
            continue;
        }

        let Ok(systems) = q_systems.get(beam.structure_entity) else {
            warn!("Structure missing `Systems` component {:?}", beam.structure_entity);
            commands.entity(beam.structure_entity).log_components();
            commands.entity(entity).insert(NeedsDespawned);
            continue;
        };

        let Ok(mut energy_storage_system) = systems.query_mut(&mut q_energy_storage_system) else {
            warn!("Structure missing `EnergyStorageSystem` system {:?}", beam.structure_entity);
            commands.entity(beam.structure_entity).log_components();

            continue;
        };

        if !energy_storage_system.decrease_energy(beam.property.energy_per_second * delta_time) {
            commands.entity(entity).insert(NeedsDespawned);
            continue;
        }

        let ray_start = g_trans.translation();
        let ray_dir = g_trans.forward();

        let Ok(Some((hit_entity, toi))) = rapier_context.cast_ray(
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
        ) else {
            continue;
        };

        let mut handle_structure = |beam_shooter_entity: Entity,
                                    structure: &Structure,
                                    // being_mined: &mut BeingMined,
                                    structure_global_trans: &GlobalTransform| {
            let global_point_hit = ray_start + (ray_dir * (toi + 0.01));

            let local_point_hit = Quat::from_affine3(&structure_global_trans.affine())
                .inverse()
                .mul_vec3(global_point_hit - structure_global_trans.translation());

            if let Ok(block_coord) =
                structure.relative_coords_to_local_coords_checked(local_point_hit.x, local_point_hit.y, local_point_hit.z)
            {
                let hit_structure_entity = structure.get_entity().expect("Missing structure entity");

                let break_delta = delta_time * beam.property.break_force;

                if let Some(block) = mining_blocks.iter_mut().find(|b| {
                    b.hit_structure_entity == hit_structure_entity
                        && b.beam_shooter_entity == beam_shooter_entity
                        && b.hit_coordinate == block_coord
                }) {
                    block.break_increase += break_delta;
                } else {
                    mining_blocks.push(CachedBlockBeingMined {
                        hit_structure_entity,
                        beam_shooter_entity,
                        hit_coordinate: block_coord,
                        break_increase: break_delta,
                    });
                }
            } else {
                warn!("Mining laser hit out of bounds coordinates?");
            }
        };

        if let Ok((structure, g_trans)) = q_structure.get(hit_entity) {
            handle_structure(beam.structure_entity, &structure, g_trans);
        } else if let Ok(parent) = q_parent.get(hit_entity) {
            let entity = parent.get();
            if let Ok((structure, g_trans)) = q_structure.get(entity) {
                handle_structure(beam.structure_entity, &structure, g_trans);
            }
        }
    }

    if !mining_blocks.is_empty() {
        println!("Doing {mining_blocks:?}");
    }

    for CachedBlockBeingMined {
        hit_structure_entity,
        beam_shooter_entity,
        hit_coordinate,
        break_increase,
    } in mining_blocks
    {
        let Ok(mut being_mined) = q_being_mined.get_mut(hit_structure_entity) else {
            error!("No being mined! Logging components of something that should be a structure but isn't.");
            commands.entity(hit_structure_entity).log_components();
            continue;
        };

        if let Some(&mining_block) = being_mined.0.get(&hit_coordinate) {
            if let Ok(mut mining_block) = q_mining_block.get_mut(mining_block) {
                mining_block.time_mined += break_increase;
                mining_block.dirty = true;
            }
        } else {
            let mining_block = commands
                .spawn((
                    Name::new("Block being mined"),
                    MiningBlock {
                        block_coord: hit_coordinate,
                        time_mined: break_increase,
                        dirty: true,
                        last_toucher: beam_shooter_entity,
                    },
                ))
                .id();

            commands.entity(beam_shooter_entity).add_child(mining_block);

            being_mined.0.insert(hit_coordinate, mining_block);
        }
    }
}

#[derive(Component)]
struct MiningBeam {
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
                let sec = time.delta_seconds();

                let world_id = physics_world.map(|bw| bw.world_id).unwrap_or_default();

                for line in mining_system.lines.iter() {
                    let energy = line.property.energy_per_second * sec;

                    if energy_storage_system.decrease_energy(energy) {
                        // AT SOME POINT, THE NEGATIVE SIGN HAS TO BE REMOVED HERE!!!!!
                        // I SHOULD NOT HAVE TO NEGATE THE DIRECTION
                        // SINCE THERE IS NO WAY TO ROTATE THE CANNONS, FOR NOW THIS HAS
                        // TO BE HERE, BUT ONCE CANNONS CAN BE ROTATED, REMOVE THIS!
                        // let beam_direction = global_transform.affine().matrix3.mul_vec3(-line.direction.direction_vec3());
                        let beam_direction = -line.direction.direction_vec3();

                        let beam_begin = line.end();
                        let rel_pos = structure.block_relative_position(beam_begin.coords());

                        let mining_beam = commands
                            .spawn((
                                Name::new("Mining beam"),
                                MiningBeam {
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
