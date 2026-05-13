//! Represents all the energy stored on a structure

use bevy::{math::bounding::Aabb3d, prelude::*};

use bevy_rapier3d::{
    dynamics::{FixedJointBuilder, ImpulseJoint, Velocity},
    geometry::{CollisionGroups, Group},
    pipeline::QueryFilter,
    plugin::{RapierContextEntityLink, ReadRapierContext},
    prelude::{GenericJointBuilder, JointAxesMask, TypedJoint},
};
use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_events::BlockMessagesSet, block_face::BlockFace},
    entities::EntityId,
    events::{block_events::BlockChangedMessage, structure::structure_event::StructureMessageIterator},
    physics::structure_physics::ChunkPhysicsPart,
    prelude::BlockCoordinate,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        events::StructureLoadedMessage,
        full_structure::FullStructure,
        shields::SHIELD_COLLISION_GROUP,
        systems::{
            StructureSystem, StructureSystemImpl, StructureSystemOrdering, StructureSystemType, StructureSystems, StructureSystemsSet,
            SystemActive,
            dock_system::{DockSystem, Docked},
        },
    },
    utils::{
        ecs::{FixedUpdateRemovedComponents, MutOrMutRef, register_fixed_update_removed_component},
        quat_math::QuatMath,
    },
};
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{DefaultPersistentComponent, PersistentComponent, make_persistent};

use super::sync::register_structure_system;

const MAX_DOCK_CHECK: f32 = 2.0;

#[derive(Component, Default, Debug, Reflect)]
pub struct DockedEntities(Vec<Entity>);

#[derive(Clone, Copy, Debug)]
pub struct DockBlock {
    id: u16,
    y_rotate: bool,
}

#[derive(Resource, Default)]
struct DockBlocks(Vec<DockBlock>);

impl DockBlocks {
    fn contains(&self, id: u16) -> bool {
        self.0.iter().any(|x| x.id == id)
    }

    fn push(&mut self, b: DockBlock) {
        self.0.push(b)
    }

    fn get(&self, id: u16) -> Option<DockBlock> {
        self.0.iter().find(|x| x.id == id).copied()
    }
}

fn dock_block_update_system(
    mut event: MessageReader<BlockChangedMessage>,
    mut system_query: Query<&mut DockSystem>,
    mut q_systems: Query<(&mut StructureSystems, &mut StructureSystemOrdering)>,
    mut commands: Commands,
    q_system: Query<&StructureSystem, With<DockSystem>>,
    registry: Res<Registry<StructureSystemType>>,
    dock_blocks: Res<DockBlocks>,
) {
    for (structure, events) in event.read().group_by_structure() {
        let Ok((mut systems, mut ordering)) = q_systems.get_mut(structure) else {
            continue;
        };

        let mut new_system_if_needed = DockSystem::default();

        let mut system = systems
            .query_mut(&mut system_query)
            .map(MutOrMutRef::from)
            .unwrap_or(MutOrMutRef::from(&mut new_system_if_needed));

        for ev in events {
            if dock_blocks.contains(ev.old_block) {
                system.block_removed(ev.block.coords());
            }

            if dock_blocks.contains(ev.new_block) {
                system.block_added(ev.block.coords());
            }
        }

        match system {
            MutOrMutRef::Mut(existing_system) => {
                if existing_system.is_empty() {
                    let system = *systems.query(&q_system).expect("This should always exist on a StructureSystem");
                    systems.remove_system(&mut commands, &system, &registry, &mut ordering);
                }
            }
            MutOrMutRef::Ref(new_system) => {
                if !new_system.is_empty() {
                    let (id, _) = systems.add_system(&mut commands, std::mem::take(&mut new_system_if_needed), &registry);
                    if let Some(system_type) = registry.from_id(DockSystem::unlocalized_name())
                        && system_type.is_activatable()
                    {
                        ordering.add_to_next_available(id);
                    }
                }
            }
        }
    }
}

fn dock_structure_loaded_event_processor(
    mut event_reader: MessageReader<StructureLoadedMessage>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    mut commands: Commands,
    registry: Res<Registry<StructureSystemType>>,
    q_dock_system: Query<(), With<DockSystem>>,
    dock_blocks: Res<DockBlocks>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            if systems.query(&q_dock_system).is_ok() {
                continue;
            }

            let mut system = DockSystem::default();

            for block in structure.all_blocks_iter(false) {
                if dock_blocks.contains(structure.block_id_at(block)) {
                    system.block_added(block);
                }
            }

            if !system.is_empty() {
                systems.add_system(&mut commands, system, &registry);
            }
        }
    }
}

#[derive(Component)]
struct JustUndocked;

fn on_active(
    context_access: ReadRapierContext,
    q_docked: Query<&Docked>,
    mut q_structure: Query<(&mut Structure, &GlobalTransform, &RapierContextEntityLink)>,
    q_active: Query<(Entity, &StructureSystem, &DockSystem, Ref<SystemActive>, Option<&JustUndocked>)>,
    q_inactive: Query<Entity, (With<DockSystem>, Without<SystemActive>, With<JustUndocked>)>,
    q_chunk_entity: Query<&ChunkPhysicsPart>,
    q_velocity: Query<&Velocity>,
    q_docked_list: Query<&DockedEntities>,
    mut commands: Commands,
    dock_blocks: Res<DockBlocks>,
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

        if active_system_flag.is_added()
            && let Ok(docked) = docked
        {
            let vel = q_velocity.get(docked.to).copied().unwrap_or_default();

            commands.entity(ss.structure_entity()).remove::<Docked>().insert(vel);
            commands.entity(system_entity).insert(JustUndocked);

            continue;
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

            let context = context_access.get(*pw);

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

            let point = hit_g_trans.to_matrix().inverse().transform_point3(moved_point);

            let Ok(hit_coords) = hit_structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) else {
                return;
            };

            let block = hit_structure.block_id_at(hit_coords);

            if !dock_blocks.contains(block) {
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

            let child_anchor = structure.block_relative_position(docking_block) + docking_look_direction.as_vec3() * 0.5;
            let parent_anchor = hit_structure.block_relative_position(hit_coords) + hit_block_direction.as_vec3() * 0.5;

            let mut rotate_x: bool = false;
            let mut rotate_y: bool = false;
            let mut rotate_z: bool = false;

            let mut rt_y = false;
            if let Some(dock_block_to) = dock_blocks.get(hit_structure.block_id_at(hit_coords)) {
                rt_y |= dock_block_to.y_rotate;
            }
            if let Some(dock_block_from) = dock_blocks.get(structure.block_id_at(docking_block)) {
                rt_y |= dock_block_from.y_rotate;
            }

            if rt_y {
                let rotation_dir = structure.block_rotation(docking_block).direction_of(BlockFace::Front);

                match rotation_dir {
                    BlockDirection::PosX | BlockDirection::NegX => rotate_x = true,
                    BlockDirection::PosY | BlockDirection::NegY => rotate_y = true,
                    BlockDirection::PosZ | BlockDirection::NegZ => rotate_z = true,
                }
            }

            // dock
            need_docked.push((
                ss.structure_entity(),
                delta_position,
                if rt_y { Quat::IDENTITY } else { delta_rotation },
                Docked {
                    rotate_x,
                    rotate_y,
                    rotate_z,
                    child_anchor,
                    parent_anchor,
                    to: structure_entity,
                    to_block: hit_coords,
                    relative_rotation: relative_docked_ship_rotation,
                    this_block: docking_block,
                    relative_translation: rel_pos,
                },
            ));

            info!("{need_docked:?}");
        }
    }

    for (entity, delta_position, delta_rotation, docked) in need_docked {
        let Ok((_, g_trans, pw)) = q_structure.get_mut(entity) else {
            unreachable!("Guarenteed because only entities that are in this list are valid structures from above for loop.");
        };

        if let Ok(to_docked) = q_docked.get(docked.to)
            && to_docked.to == entity
        {
            // The other ship is already docked to this - don't re-dock.
            continue;
        }

        let context = context_access.get(*pw);

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

        context.intersect_aabb_conservative(aabb, Default::default(), |e| {
            if let Ok(ce) = q_chunk_entity.get(e)
                && ce.structure_entity != entity
                && !check_docked_entities(&ce.structure_entity, &q_docked_list, &e)
            {
                hit_something_bad = true;
            }

            true
        });

        // TODO: Re-enable this check but include child docked structures aswell
        // if !hit_something_bad {
        commands.entity(entity).insert(docked);
        // } else {
        // info!("Hit something bad!");
        // }
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
    q_docked: Query<(Entity, &Docked)>,
    mut block_change_reader: MessageReader<BlockChangedMessage>,
    q_velocity: Query<&Velocity>,
    mut commands: Commands,
    dock_blocks: Res<DockBlocks>,
) {
    for ev in block_change_reader.read() {
        if !dock_blocks.contains(ev.old_block) {
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
    removed_docks_reader: FixedUpdateRemovedComponents<Docked>,
    q_added_dock: Query<(Entity, &Docked), Changed<Docked>>,
    mut q_docked_list: Query<&mut DockedEntities>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
) {
    for removed_dock_ent in removed_docks_reader.read() {
        if let Ok(mut ecmds) = commands.get_entity(removed_dock_ent) {
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

        let Ok([structure_to, structure_from]) = q_structure.get_many([docked.to, ent]) else {
            error!("Invalid dock ent(s)!");
            continue;
        };

        let joint = if docked.rotate_y || docked.rotate_x || docked.rotate_z {
            let axis_to = structure_to
                .block_rotation(docked.to_block)
                .direction_of(BlockFace::Front)
                .as_vec3()
                .normalize();

            let axis_from = structure_from
                .block_rotation(docked.this_block)
                .direction_of(BlockFace::Front)
                .as_vec3()
                .normalize();

            let joint = GenericJointBuilder::new(JointAxesMask::LOCKED_REVOLUTE_AXES)
                .local_anchor1(docked.parent_anchor)
                .local_anchor2(docked.child_anchor)
                .local_axis1(axis_to)
                // Negative because the the fronts face each other
                .local_axis2(-axis_from)
                .build();

            ImpulseJoint::new(docked.to, TypedJoint::GenericJoint(joint))
        } else {
            let joint = FixedJointBuilder::default()
                .local_anchor1(docked.relative_translation)
                .local_basis1(docked.relative_rotation)
                .build();

            ImpulseJoint::new(docked.to, joint)
        };

        commands.entity(ent).insert(joint);
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

impl DefaultPersistentComponent for DockSystem {}

fn add_dock_blocks(mut dock_blocks: ResMut<DockBlocks>, blocks: Res<Registry<Block>>) {
    if let Some(ship_dock) = blocks.from_id("cosmos:ship_dock") {
        dock_blocks.push(DockBlock {
            id: ship_dock.id(),
            y_rotate: false,
        });
    }
    if let Some(ship_dock) = blocks.from_id("cosmos:pan_dock") {
        dock_blocks.push(DockBlock {
            id: ship_dock.id(),
            y_rotate: true,
        });
    }
    // if let Some(ship_dock) = blocks.from_id("cosmos:pan_tilt_dock") {
    //     dock_blocks.push(DockBlock {
    //         id: ship_dock.id(),
    //         y_rotate: true,
    //     });
    // }
}

#[derive(Serialize, Deserialize)]
pub struct DockedPersisted {
    /// The entity this is docked to
    pub to: EntityId,
    /// The block on the entity it is docked to that acts as the docking block
    pub to_block: BlockCoordinate,
    /// The block on this entity that acts as the docking block
    pub this_block: BlockCoordinate,

    /// Relative to entity we are docked to
    pub relative_rotation: Quat,
    /// Relative translation to the entity we are docked to
    pub relative_translation: Vec3,

    /// If this docked ship can rotate about this axis relative to them
    pub rotate_x: bool,
    /// If this docked ship can rotate about this axis relative to them
    pub rotate_y: bool,
    /// If this docked ship can rotate about this axis relative to them
    pub rotate_z: bool,

    /// Where (relative to the parent) this ship is docked/anchored to. Rotations will be made
    /// about this anchor
    pub parent_anchor: Vec3,
    /// Where (relative to itself) this ship is docked/anchored to.Rotations will be made
    /// about this anchor
    pub child_anchor: Vec3,
}

impl PersistentComponent for Docked {
    type SaveType = DockedPersisted;

    fn convert_to_save_type<'a>(
        &'a self,
        q_entity_ids: &Query<&EntityId>,
    ) -> Option<cosmos_core::utils::ownership::MaybeOwned<'a, Self::SaveType>> {
        let id = *q_entity_ids.get(self.to).ok()?;

        Some(
            DockedPersisted {
                to: id,
                to_block: self.to_block,
                rotate_x: self.rotate_x,
                rotate_y: self.rotate_y,
                rotate_z: self.rotate_z,
                this_block: self.this_block,
                child_anchor: self.child_anchor,
                parent_anchor: self.parent_anchor,
                relative_rotation: self.relative_rotation,
                relative_translation: self.relative_translation,
            }
            .into(),
        )
    }

    fn convert_from_save_type(
        save_type: Self::SaveType,
        entity_id_manager: &crate::persistence::make_persistent::EntityIdManager,
    ) -> Option<Self> {
        let to = entity_id_manager.entity_from_entity_id(&save_type.to)?;

        Some(Docked {
            to,
            to_block: save_type.to_block,
            rotate_x: save_type.rotate_x,
            rotate_y: save_type.rotate_y,
            rotate_z: save_type.rotate_z,
            this_block: save_type.this_block,
            child_anchor: save_type.child_anchor,
            parent_anchor: save_type.parent_anchor,
            relative_rotation: save_type.relative_rotation,
            relative_translation: save_type.relative_translation,
        })
    }

    fn initialize(&mut self, _self_entity: Entity, _commands: &mut Commands) {}
}

pub(super) fn register(app: &mut App) {
    make_persistent::<DockSystem>(app);
    make_persistent::<Docked>(app);
    register_fixed_update_removed_component::<Docked>(app);

    app.add_systems(
        FixedUpdate,
        (
            dock_structure_loaded_event_processor
                .in_set(StructureSystemsSet::InitSystems)
                .ambiguous_with(StructureSystemsSet::InitSystems)
                .run_if(in_state(GameState::Playing)),
            (
                dock_block_update_system
                    .in_set(BlockMessagesSet::ProcessMessages)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
                on_active, //.after(ThrusterSystemSet::ApplyThrusters),
                monitor_removed_dock_blocks
                    //.after(ThrusterSystemSet::ApplyThrusters) // velocity is changed in `ApplyThrusters`, which is needed here.
                    .in_set(BlockMessagesSet::ProcessMessages)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
                add_dock_list,
                add_dock_properties,
            )
                .chain()
                .in_set(BlockMessagesSet::ProcessMessages)
                .run_if(in_state(GameState::Playing)),
        ),
    )
    .add_systems(OnEnter(GameState::PostLoading), add_dock_blocks)
    .register_type::<DockedEntities>()
    .init_resource::<DockBlocks>();

    register_structure_system::<DockSystem>(app, true, "cosmos:ship_dock");
}
