//! Represents all the energy stored on a structure

use bevy::{
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        query::{Changed, With, Without},
        removal_detection::RemovedComponents,
        world::Ref,
    },
    math::{bounding::Aabb3d, Quat, Vec3},
    prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, Query, Res, Update},
    reflect::Reflect,
    transform::components::GlobalTransform,
};

use bevy_rapier3d::{
    dynamics::{FixedJointBuilder, ImpulseJoint, Velocity},
    geometry::{CollisionGroups, Group},
    pipeline::QueryFilter,
    plugin::{RapierContextAccess, RapierContextEntityLink},
};
use cosmos_core::{
    block::{block_events::BlockEventsSet, block_face::BlockFace, Block},
    events::block_events::BlockChangedEvent,
    physics::structure_physics::ChunkPhysicsPart,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::{
        events::StructureLoadedEvent,
        full_structure::FullStructure,
        shields::SHIELD_COLLISION_GROUP,
        systems::{
            dock_system::{DockSystem, Docked},
            StructureSystem, StructureSystemType, StructureSystems, StructureSystemsSet, SystemActive,
        },
        Structure,
    },
    utils::quat_math::QuatMath,
};

use super::{sync::register_structure_system, thruster_system::ThrusterSystemSet};

const MAX_DOCK_CHECK: f32 = 1.3;

#[derive(Component, Default, Debug, Reflect)]
pub struct DockedEntities(Vec<Entity>);

fn dock_block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut DockSystem>,
    q_systems: Query<&StructureSystems>,
) {
    for ev in event.read() {
        let Ok(systems) = q_systems.get(ev.block.structure()) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut system_query) else {
            continue;
        };

        if blocks.from_numeric_id(ev.old_block).unlocalized_name() == "cosmos:ship_dock" {
            system.block_removed(ev.block.coords());
        }

        if blocks.from_numeric_id(ev.new_block).unlocalized_name() == "cosmos:ship_dock" {
            system.block_added(ev.block.coords());
        }
    }
}

fn dock_structure_loaded_event_processor(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = DockSystem::default();

            for block in structure.all_blocks_iter(false) {
                if structure.block_at(block, &blocks).unlocalized_name() == "cosmos:ship_dock" {
                    system.block_added(block);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

#[derive(Component)]
struct JustUndocked;

fn on_active(
    context_access: RapierContextAccess,
    q_docked: Query<&Docked>,
    mut q_structure: Query<(&mut Structure, &GlobalTransform, &RapierContextEntityLink)>,
    q_active: Query<(Entity, &StructureSystem, &DockSystem, Ref<SystemActive>, Option<&JustUndocked>)>,
    q_inactive: Query<Entity, (With<DockSystem>, Without<SystemActive>, With<JustUndocked>)>,
    q_chunk_entity: Query<&ChunkPhysicsPart>,
    blocks: Res<Registry<Block>>,
    q_velocity: Query<&Velocity>,
    q_docked_list: Query<&DockedEntities>,
    mut commands: Commands,
) {
    for e in q_inactive.iter() {
        commands.entity(e).remove::<JustUndocked>();
    }

    let mut need_docked = vec![];

    for (system_entity, ss, ds, active_system_flag, just_undocked) in q_active.iter() {
        let Ok((structure, g_trans, pw)) = q_structure.get(ss.structure_entity()) else {
            continue;
        };

        let docked = q_docked.get(ss.structure_entity());

        if active_system_flag.is_added() {
            if let Ok(docked) = docked {
                let vel = q_velocity.get(docked.to).copied().unwrap_or_default();

                commands.entity(ss.structure_entity()).remove::<Docked>().insert(vel);
                commands.entity(system_entity).insert(JustUndocked);

                continue;
            }
        }

        if docked.is_ok() || just_undocked.is_some() {
            continue;
        }

        for &docking_block in ds.block_locations() {
            let rel_pos = structure.block_relative_position(docking_block);
            let block_rotation = structure.block_rotation(docking_block);
            let docking_look_direction = block_rotation.direction_of(BlockFace::Front);
            let front_direction = docking_look_direction.as_vec3();

            let abs_block_pos = g_trans.transform_point(rel_pos);

            let my_rotation = Quat::from_affine3(&g_trans.affine());
            let ray_dir = my_rotation.mul_vec3(front_direction);

            let context = context_access.context(pw);

            let Some((entity, intersection)) = context.cast_ray_and_get_normal(
                abs_block_pos,
                ray_dir,
                MAX_DOCK_CHECK,
                false,
                QueryFilter::new()
                    .groups(CollisionGroups::new(
                        Group::ALL & !SHIELD_COLLISION_GROUP,
                        Group::ALL & !SHIELD_COLLISION_GROUP,
                    ))
                    .predicate(&|e| {
                        let Ok(ce) = q_chunk_entity.get(e) else {
                            return false;
                        };

                        ce.structure_entity != ss.structure_entity()
                    }),
            ) else {
                continue;
            };

            let Ok(structure_entity) = q_chunk_entity.get(entity).map(|x| x.structure_entity) else {
                continue;
            };

            let Ok((hit_structure, hit_g_trans, _)) = q_structure.get(structure_entity) else {
                return;
            };

            let moved_point = intersection.point - intersection.normal * 0.01;

            let point = hit_g_trans.compute_matrix().inverse().transform_point3(moved_point);

            let Ok(hit_coords) = hit_structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) else {
                return;
            };

            let block: &Block = hit_structure.block_at(hit_coords, &blocks);

            if block.unlocalized_name() != "cosmos:ship_dock" {
                continue;
            };

            let hit_block_direction = hit_structure.block_rotation(hit_coords).direction_of(BlockFace::Front);
            let hit_rotation = Quat::from_affine3(&hit_g_trans.affine());
            let front_direction = hit_rotation.mul_vec3(hit_block_direction.as_vec3());

            let dotted = ray_dir.dot(front_direction);

            if dotted > -0.92 {
                continue;
            }

            let relative_docked_ship_rotation = snap_to_right_angle(hit_rotation.inverse() * my_rotation);

            let my_new_abs_rotation = hit_rotation * relative_docked_ship_rotation;
            let delta_rotation = my_new_abs_rotation.inverse() * my_rotation;

            let rel_pos = hit_structure.block_relative_position(hit_coords)
                - relative_docked_ship_rotation
                    .mul_vec3(structure.block_relative_position(docking_block) + docking_look_direction.as_vec3());

            let delta_position = rel_pos - (g_trans.translation() - hit_g_trans.translation());

            // dock
            need_docked.push((
                ss.structure_entity(),
                delta_position,
                delta_rotation,
                Docked {
                    to: structure_entity,
                    to_block: hit_coords,
                    relative_rotation: relative_docked_ship_rotation,
                    this_block: docking_block,
                    relative_translation: rel_pos,
                },
            ));
        }
    }

    for (entity, delta_position, delta_rotation, docked) in need_docked {
        let Ok((_, g_trans, pw)) = q_structure.get_mut(entity) else {
            unreachable!("Guarenteed because only entities that are in this list are valid structures from above for loop.");
        };

        let context = context_access.context(pw);

        let (min, max) = computed_total_aabb(
            entity,
            delta_rotation.inverse(),
            delta_position,
            &q_docked_list,
            g_trans.translation(),
            &mut q_structure,
        );

        let aabb = Aabb3d {
            min: (min + Vec3::splat(0.1)).into(),
            max: (max - Vec3::splat(0.1)).into(),
        };

        let mut hit_something_bad = false;

        context.colliders_with_aabb_intersecting_aabb(aabb, |e| {
            if let Ok(ce) = q_chunk_entity.get(e) {
                if ce.structure_entity != entity && !check_docked_entities(&ce.structure_entity, &q_docked_list, &e) {
                    hit_something_bad = true;
                }
            }

            true
        });

        if !hit_something_bad {
            commands.entity(entity).insert(docked);
        }
    }
}

fn check_docked_entities(looking_at: &Entity, q_docked_list: &Query<&DockedEntities>, searching_for: &Entity) -> bool {
    let Ok(dl) = q_docked_list.get(*looking_at) else {
        return false;
    };

    for e in dl.0.iter() {
        if e == searching_for {
            return true;
        }
        if check_docked_entities(e, q_docked_list, searching_for) {
            return true;
        }
    }

    false
}

fn computed_total_aabb(
    entity: Entity,
    delta_rotation: Quat,
    delta_position: Vec3,
    q_docked_list: &Query<&DockedEntities>,
    base_translation: Vec3,
    q_structure: &mut Query<(&mut Structure, &GlobalTransform, &RapierContextEntityLink)>,
) -> (Vec3, Vec3) {
    let Ok((mut structure, g_trans, _)) = q_structure.get_mut(entity) else {
        unreachable!("Guarenteed because only entities that are in this list are valid structures from above for loop.");
    };

    let Some((min_block_bounds, max_block_bounds)) = FullStructure::placed_block_bounds(&mut structure) else {
        return (Vec3::ZERO, Vec3::ZERO);
    };

    let trans = g_trans.translation() + delta_position;
    let rot = Quat::from_affine3(&g_trans.affine()) * delta_rotation;

    let (mut min, mut max) = (
        trans + (rot * (structure.block_relative_position(min_block_bounds) - Vec3::splat(0.5))) - base_translation,
        trans + (rot * (structure.block_relative_position(max_block_bounds) + Vec3::splat(0.5))) - base_translation,
    );

    // Due to rotations, these can become swapped
    let actual_min = min.min(max);
    let actual_max = min.max(max);
    min = actual_min;
    max = actual_max;

    if let Ok(docked_list) = q_docked_list.get(entity) {
        for ent in docked_list.0.iter() {
            let (their_min, their_max) =
                computed_total_aabb(*ent, delta_rotation, delta_position, q_docked_list, base_translation, q_structure);
            min = their_min.min(min);
            max = their_max.max(max);
        }
    }

    (min, max)
}

fn monitor_removed_dock_blocks(
    blocks: Res<Registry<Block>>,
    q_docked: Query<(Entity, &Docked)>,
    mut block_change_reader: EventReader<BlockChangedEvent>,
    q_velocity: Query<&Velocity>,
    mut commands: Commands,
) {
    for ev in block_change_reader.read() {
        if blocks.from_numeric_id(ev.old_block).unlocalized_name() != "cosmos:ship_dock" {
            continue;
        }

        for (docked_entity, docked) in q_docked.iter() {
            if docked.to == ev.block.structure() && docked.to_block == ev.block.coords()
                || ev.block.structure() == docked_entity && docked.this_block == ev.block.coords()
            {
                let vel = q_velocity.get(docked.to).copied().unwrap_or_default();
                commands.entity(docked_entity).remove::<Docked>().insert(vel);
            }
        }
    }
}

fn add_dock_list(mut commands: Commands, q_needs_list: Query<Entity, (With<Structure>, Without<DockedEntities>)>) {
    for e in q_needs_list.iter() {
        commands.entity(e).insert(DockedEntities::default());
    }
}

fn add_dock_properties(
    mut removed_docks_reader: RemovedComponents<Docked>,
    q_added_dock: Query<(Entity, &Docked), Changed<Docked>>,
    mut q_docked_list: Query<&mut DockedEntities>,
    mut commands: Commands,
) {
    for removed_dock_ent in removed_docks_reader.read() {
        if let Some(mut ecmds) = commands.get_entity(removed_dock_ent) {
            ecmds.remove::<ImpulseJoint>();
        }

        for mut docked_list in q_docked_list.iter_mut() {
            if let Some((idx, _)) = docked_list.0.iter().enumerate().find(|(_, e)| **e == removed_dock_ent) {
                docked_list.0.swap_remove(idx);
            }
        }
    }

    for (ent, docked) in q_added_dock.iter() {
        for mut docked_list in q_docked_list.iter_mut() {
            if let Some((idx, _)) = docked_list.0.iter().enumerate().find(|(_, e)| **e == ent) {
                docked_list.0.swap_remove(idx);
            }
        }

        let Ok(mut entity_docking_to_list) = q_docked_list.get_mut(docked.to) else {
            panic!(
                "Entity {:?} missing DockedEntities list but was attempted to be docked to!",
                docked.to
            );
        };
        entity_docking_to_list.0.push(ent);

        let joint = FixedJointBuilder::default()
            .local_anchor1(docked.relative_translation)
            .local_basis1(docked.relative_rotation);

        commands.entity(ent).insert(ImpulseJoint::new(docked.to, joint));
    }
}

/// Takes a rotation and returns the rotation that is the closest with all axes pointing at right angle intervals
fn snap_to_right_angle(rot: Quat) -> Quat {
    let nearest_forward = nearest_axis(rot * Vec3::Z);
    let nearest_up = nearest_axis(rot * Vec3::Y);
    // return Quat::look_to(nearest_forward, nearest_up);
    Quat::looking_to(-nearest_forward, nearest_up)
}

/// Find the absolute axis that is closest to the given direction
fn nearest_axis(direction: Vec3) -> Vec3 {
    let x = direction.x.abs();
    let y = direction.y.abs();
    let z = direction.z.abs();
    if x > y && x > z {
        Vec3::new(direction.x.signum(), 0.0, 0.0)
    } else if y > x && y > z {
        Vec3::new(0.0, direction.y.signum(), 0.0)
    } else {
        Vec3::new(0.0, 0.0, direction.z.signum())
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            dock_structure_loaded_event_processor
                .in_set(StructureSystemsSet::InitSystems)
                .ambiguous_with(StructureSystemsSet::InitSystems)
                .run_if(in_state(GameState::Playing)),
            (
                dock_block_update_system
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
                on_active.after(ThrusterSystemSet::ApplyThrusters),
                monitor_removed_dock_blocks
                    .after(ThrusterSystemSet::ApplyThrusters) // velocity is changed in `ApplyThrusters`, which is needed here.
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
                add_dock_list,
                add_dock_properties,
            )
                .chain()
                .in_set(BlockEventsSet::ProcessEvents)
                .run_if(in_state(GameState::Playing)),
        ),
    )
    .register_type::<DockedEntities>();

    register_structure_system::<DockSystem>(app, true, "cosmos:ship_dock");
}
