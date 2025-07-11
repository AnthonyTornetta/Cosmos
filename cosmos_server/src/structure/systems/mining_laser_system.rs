use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rapier3d::{
    geometry::{CollisionGroups, Group},
    pipeline::QueryFilter,
    plugin::{RapierContextEntityLink, ReadRapierContext},
};
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockBreakEvent, BlockEventsSet},
        blocks::fluid::FLUID_COLLISION_GROUP,
    },
    ecs::NeedsDespawned,
    netty::NoSendEntity,
    physics::location::LocationPhysicsSet,
    prelude::Station,
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        coordinates::BlockCoordinate,
        shared::{DespawnWithStructure, MeltingDown},
        shields::SHIELD_COLLISION_GROUP,
        ship::Ship,
        structure_block::StructureBlock,
        systems::{
            StructureSystem, StructureSystems, StructureSystemsSet, SystemActive,
            energy_storage_system::EnergyStorageSystem,
            line_system::LineBlocks,
            mining_laser_system::{MiningLaserProperty, MiningLaserPropertyCalculator, MiningLaserSystem},
        },
    },
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::{line_system::add_line_system, sync::register_structure_system};

const BEAM_MAX_RANGE: f32 = 250.0;
const BREAK_DECAY_RATIO: f32 = 1.5;

#[derive(Component, Debug)]
/// If this is on a structure, the mining laser will not mine this
pub struct CannotBeMinedByMiningLaser;

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
    mut q_mining_blocks: Query<(Entity, &mut MiningBlock)>,
    mut ev_writer: EventWriter<BlockBreakEvent>,
    blocks: Res<Registry<Block>>,
    time: Res<Time>,
) {
    let delta = time.delta_secs();

    for (structure_entity, structure, mut being_mined) in q_structure.iter_mut() {
        being_mined.0.retain(|coordinate, &mut entity| {
            let Ok((_, mut mining_block)) = q_mining_blocks.get_mut(entity) else {
                return false;
            };

            let block = structure.block_at(mining_block.block_coord, &blocks);

            if mining_block.time_mined >= block.mining_resistance() {
                ev_writer.write(BlockBreakEvent {
                    block: StructureBlock::new(*coordinate, structure_entity),
                    breaker: mining_block.last_toucher,
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

    q_mining_blocks.iter().for_each(|(entity, mining_block)| {
        if mining_block.time_mined <= 0.0 {
            commands.entity(entity).insert(NeedsDespawned);
        }
    });
}

fn update_mining_beams(
    mut commands: Commands,
    mut q_mining_beams: Query<(Entity, &mut MiningBeam, &RapierContextEntityLink, &GlobalTransform)>,
    q_systems: Query<&StructureSystems>,
    mut q_energy_storage_system: Query<&mut EnergyStorageSystem>,
    q_structure: Query<(&Structure, &GlobalTransform), Without<CannotBeMinedByMiningLaser>>,
    mut q_mining_block: Query<&mut MiningBlock>,
    mut q_being_mined: Query<&mut BeingMined>,
    q_is_system_active: Query<(), With<SystemActive>>,
    rapier_context_access: ReadRapierContext,
    q_parent: Query<&ChildOf>,
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

    let delta_time = time.delta_secs();

    // group by structure

    let mut structure_beams: HashMap<Entity, Vec<_>> = HashMap::new();

    for (beam_ent, beam, p_world, g_trans) in q_mining_beams.iter_mut() {
        if !q_is_system_active.contains(beam.system_entity) {
            commands.entity(beam_ent).insert(NeedsDespawned);
            return;
        }

        structure_beams
            .entry(beam.structure_entity)
            .or_default()
            .push((beam_ent, beam, p_world, g_trans));
    }

    let le_beams = structure_beams
        .into_iter()
        .flat_map(|(structure, mut beams)| {
            let Ok(systems) = q_systems.get(structure) else {
                warn!("Structure missing `Systems` component {:?}", structure);
                return None;
            };

            let Ok(mut energy_storage_system) = systems.query_mut(&mut q_energy_storage_system) else {
                warn!("Structure missing `EnergyStorageSystem` system {:?}", structure);
                return None;
            };

            beams.retain(|(beam_ent, beam, _, _)| {
                if energy_storage_system.decrease_energy(beam.property.energy_per_second * delta_time) != 0.0 {
                    commands.entity(*beam_ent).insert(NeedsDespawned);
                    return false;
                }
                true
            });

            Some(beams)
        })
        .flatten()
        .collect::<Vec<_>>();

    let hits = le_beams
        .par_iter()
        .flat_map(|(_, beam, p_world, g_trans)| {
            let ray_start = g_trans.translation();
            let ray_dir = g_trans.forward();

            let rapier_context = rapier_context_access.get(**p_world);

            let (hit_entity, toi) = rapier_context.cast_ray(
                ray_start,
                ray_dir.into(),
                BEAM_MAX_RANGE,
                true,
                QueryFilter::predicate(QueryFilter::default(), &|entity| {
                    if beam.structure_entity == entity {
                        false
                    } else if let Ok(parent) = q_parent.get(entity) {
                        parent.parent() != beam.structure_entity
                    } else {
                        false
                    }
                })
                .groups(CollisionGroups::new(
                    Group::ALL & !(SHIELD_COLLISION_GROUP | FLUID_COLLISION_GROUP),
                    Group::ALL & !(SHIELD_COLLISION_GROUP | FLUID_COLLISION_GROUP),
                )),
            )?;

            if let Ok((structure, g_trans)) = q_structure.get(hit_entity) {
                return Some((beam, structure, g_trans, ray_start, ray_dir, toi));
            } else if let Ok(parent) = q_parent.get(hit_entity) {
                let entity = parent.parent();
                if let Ok((structure, g_trans)) = q_structure.get(entity) {
                    return Some((beam, structure, g_trans, ray_start, ray_dir, toi));
                }
            }

            None
        })
        .collect::<Vec<_>>();

    for (beam, structure, g_trans, ray_start, ray_dir, toi) in hits {
        let beam_shooter_entity = beam.structure_entity;
        let structure_global_trans = g_trans;

        let global_point_hit = ray_start + (ray_dir * (toi + 0.01));

        let local_point_hit = Quat::from_affine3(&structure_global_trans.affine())
            .inverse()
            .mul_vec3(global_point_hit - structure_global_trans.translation());

        if let Ok(block_coord) = structure.relative_coords_to_local_coords_checked(local_point_hit.x, local_point_hit.y, local_point_hit.z)
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
                    DespawnWithStructure,
                    NoSendEntity,
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
    systems: Query<(Entity, &StructureSystems, &Structure, &RapierContextEntityLink)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (system_entity, mining_system, system) in query.iter_mut() {
        if let Ok((ship_entity, systems, structure, physics_world)) = systems.get(system.structure_entity())
            && let Ok(mut energy_storage_system) = systems.query_mut(&mut es_query)
        {
            let sec = time.delta_secs();

            for line in mining_system.lines.iter() {
                let energy = line.property.energy_per_second * sec;

                if energy_storage_system.decrease_energy(energy) == 0.0 {
                    let beam_direction = line.direction.as_vec3();

                    let beam_begin = line.end();
                    let rel_pos = structure.block_relative_position(beam_begin);

                    let mining_beam = commands
                        .spawn((
                            Name::new("Mining beam"),
                            NoSendEntity,
                            MiningBeam {
                                property: line.property,
                                structure_entity: ship_entity,
                                system_entity,
                            },
                            DespawnWithStructure,
                            Transform::from_translation(rel_pos).looking_to(beam_direction, Vec3::Y),
                            *physics_world,
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

fn register_laser_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LineBlocks<MiningLaserProperty>>) {
    if let Some(block) = blocks.from_id("cosmos:plasma_drill") {
        cannon.insert(
            block,
            MiningLaserProperty {
                energy_per_second: 50.0,
                break_force: 1.0,
            },
        )
    }
}

fn dont_mine_alive_ships(
    mut commands: Commands,
    q_ships_and_stations: Query<
        Entity,
        (
            Or<(With<Ship>, With<Station>)>,
            Without<MeltingDown>,
            Without<CannotBeMinedByMiningLaser>,
        ),
    >,
    q_melting_ships_and_stations: Query<Entity, (Or<(With<Ship>, With<Station>)>, With<MeltingDown>, With<CannotBeMinedByMiningLaser>)>,
) {
    for ent in q_ships_and_stations.iter() {
        commands.entity(ent).insert(CannotBeMinedByMiningLaser);
    }

    for ent in q_melting_ships_and_stations.iter() {
        commands.entity(ent).remove::<CannotBeMinedByMiningLaser>();
    }
}

pub(super) fn register(app: &mut App) {
    add_line_system::<MiningLaserProperty, MiningLaserPropertyCalculator>(app);

    app.add_systems(
        Update,
        (
            dont_mine_alive_ships,
            add_being_mined,
            on_activate_system,
            update_mining_beams,
            check_should_break,
        )
            .chain()
            .in_set(StructureSystemsSet::UpdateSystemsBlocks)
            .before(BlockEventsSet::PreProcessEvents)
            .after(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::PostLoading), register_laser_blocks);

    register_structure_system::<MiningLaserSystem>(app, true, "cosmos:plasma_drill");
}
