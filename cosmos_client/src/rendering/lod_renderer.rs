//! Responsible for rendering planet LODs.
//!
//! The code in this file is very smelly.
//!
//! I'm sorry. I'll fix it when I feel inspired.

use crate::{
    asset::{
        asset_loading::BlockTextureIndex,
        materials::{AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType, MaterialsSystemSet},
    },
    block::lighting::BlockLighting,
};
use bevy::{
    platform::collections::HashSet,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use cosmos_core::{
    block::{
        Block,
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
    },
    ecs::{NeedsDespawned, add_statebound_resource},
    prelude::{BlockCoordinate, UnboundBlockCoordinate},
    registry::{
        ReadOnlyRegistry, Registry,
        many_to_one::{ManyToOneRegistry, ReadOnlyManyToOneRegistry},
    },
    state::GameState,
    structure::{
        ChunkState, Structure,
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF},
        coordinates::{ChunkCoordinate, CoordinateType, UnboundCoordinateType},
        lod::{Lod, LodComponent},
        shared::DespawnWithStructure,
    },
};
use futures_lite::future;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::VecDeque,
    mem::swap,
    sync::{Arc, Mutex},
};

use super::{
    BlockMeshRegistry, LodMeshBuilder, ReadOnlyBlockMeshRegistry,
    structure_renderer::{
        BlockRenderingModes,
        chunk_rendering::{ChunkMesh, chunk_renderer::ChunkRenderer, lod_rendering::LodChunkRenderingChecker},
    },
};

#[derive(Component, Debug, Reflect, Default, Deref, DerefMut)]
struct LodMeshes(Vec<Entity>);

fn recursively_process_lod(
    lod_root: &Lod,
    lod_root_scale: CoordinateType,
    scale: CoordinateType,
    current_lod: &Lod,
    negative_most_coord: BlockCoordinate,
    offset: Vec3,
    to_process: &Mutex<Option<Vec<(ChunkMesh, Vec3, CoordinateType)>>>,
    blocks: &Registry<Block>,
    materials: &ManyToOneRegistry<Block, BlockMaterialMapping>,
    meshes_registry: &BlockMeshRegistry,
    block_textures: &Registry<BlockTextureIndex>,
    materials_registry: &Registry<MaterialDefinition>,
    lighting: &Registry<BlockLighting>,
    rendering_modes: &BlockRenderingModes,
) {
    match current_lod {
        Lod::None => {}
        Lod::Children(children) => {
            children.par_iter().enumerate().for_each(|(i, child_lod)| {
                let s4 = scale as f32 / 4.0;
                let s2 = scale / 2;

                let nmc = negative_most_coord;

                let nmc_delta = s2 * CHUNK_DIMENSIONS;

                let (offset, negative_most_coord) = match i {
                    0 => (offset + Vec3::new(-s4, -s4, -s4), nmc),
                    1 => (offset + Vec3::new(-s4, -s4, s4), nmc + BlockCoordinate::new(0, 0, nmc_delta)),
                    2 => (offset + Vec3::new(s4, -s4, s4), nmc + BlockCoordinate::new(nmc_delta, 0, nmc_delta)),
                    3 => (offset + Vec3::new(s4, -s4, -s4), nmc + BlockCoordinate::new(nmc_delta, 0, 0)),
                    4 => (offset + Vec3::new(-s4, s4, -s4), nmc + BlockCoordinate::new(0, nmc_delta, 0)),
                    5 => (offset + Vec3::new(-s4, s4, s4), nmc + BlockCoordinate::new(0, nmc_delta, nmc_delta)),
                    6 => (
                        offset + Vec3::new(s4, s4, s4),
                        nmc + BlockCoordinate::new(nmc_delta, nmc_delta, nmc_delta),
                    ),
                    7 => (offset + Vec3::new(s4, s4, -s4), nmc + BlockCoordinate::new(nmc_delta, nmc_delta, 0)),
                    _ => unreachable!(),
                };

                recursively_process_lod(
                    lod_root,
                    lod_root_scale,
                    s2,
                    child_lod,
                    negative_most_coord,
                    offset,
                    to_process,
                    blocks,
                    materials,
                    meshes_registry,
                    block_textures,
                    materials_registry,
                    lighting,
                    rendering_modes,
                );
            });
        }
        Lod::Single(lod_chunk, dirty) => {
            if !*dirty {
                return;
            }

            let mut renderer = ChunkRenderer::<LodMeshBuilder>::new();

            let lod_rendering_backend = LodChunkRenderingChecker {
                lod_root_scale,
                negative_most_coord,
                scale,
                lod_root,
            };

            renderer.render(
                materials,
                materials_registry,
                lighting,
                lod_chunk.as_ref(),
                blocks,
                meshes_registry,
                rendering_modes,
                block_textures,
                &lod_rendering_backend,
                scale as f32,
                Vec3::ZERO,
                true,
            );

            let mut mutex = to_process.lock().expect("Error locking to_process vec!");

            mutex.as_mut().unwrap().push((
                renderer.create_mesh(),
                Vec3::new(
                    offset.x * CHUNK_DIMENSIONSF,
                    offset.y * CHUNK_DIMENSIONSF,
                    offset.z * CHUNK_DIMENSIONSF,
                ),
                scale as CoordinateType,
            ));
        }
    };
}

/// If an LOD chunk is dirty, and is rerendered, its neighbors may have faces being culled that shouldn't be
/// culled. Thus, we need to mark adjacent LODs as dirty to ensure everything is rendered properly.
fn mark_adjacent_chunks_dirty(root_lod: &mut Lod, lod_root_scale: CoordinateType) {
    let mut to_make_dirty = vec![];
    find_adjacent_neighbors_that_need_dirty_flag(root_lod, &mut to_make_dirty, BlockCoordinate::ZERO, lod_root_scale);

    for coords in to_make_dirty {
        root_lod.mark_dirty(coords, lod_root_scale);
    }
}

/// If an LOD chunk is dirty, and is rerendered, its neighbors may have faces being culled that shouldn't be
/// culled. Thus, we need to mark adjacent LODs as dirty to ensure everything is rendered properly.
fn find_adjacent_neighbors_that_need_dirty_flag(
    lod: &Lod,
    coords_to_make_dirty: &mut Vec<BlockCoordinate>,
    negative_most_coord: BlockCoordinate,
    scale: CoordinateType,
) {
    let s2 = scale / 2;

    match lod {
        Lod::None => {}
        Lod::Children(children) => {
            children.iter().enumerate().for_each(|(i, c)| {
                let nmc_delta = s2 * CHUNK_DIMENSIONS;

                debug_assert_ne!(nmc_delta, 0);

                let negative_most_coord = match i {
                    0 => negative_most_coord,
                    1 => negative_most_coord + BlockCoordinate::new(0, 0, nmc_delta),
                    2 => negative_most_coord + BlockCoordinate::new(nmc_delta, 0, nmc_delta),
                    3 => negative_most_coord + BlockCoordinate::new(nmc_delta, 0, 0),
                    4 => negative_most_coord + BlockCoordinate::new(0, nmc_delta, 0),
                    5 => negative_most_coord + BlockCoordinate::new(0, nmc_delta, nmc_delta),
                    6 => negative_most_coord + BlockCoordinate::new(nmc_delta, nmc_delta, nmc_delta),
                    7 => negative_most_coord + BlockCoordinate::new(nmc_delta, nmc_delta, 0),
                    _ => unreachable!(),
                };

                find_adjacent_neighbors_that_need_dirty_flag(c, coords_to_make_dirty, negative_most_coord, s2);
            });
        }
        Lod::Single(_, dirty) => {
            if !*dirty {
                return;
            }

            const N_CHECKS: CoordinateType = 2;

            let sn = scale / N_CHECKS;

            for direction in ALL_BLOCK_DIRECTIONS {
                let Ok(negative_most_coord) = BlockCoordinate::try_from(negative_most_coord + direction.to_coordinates()) else {
                    continue;
                };

                match direction {
                    BlockDirection::NegX => {
                        for dz in 0..N_CHECKS {
                            for dy in 0..N_CHECKS {
                                if let Ok(coord) = BlockCoordinate::try_from(
                                    negative_most_coord
                                        + UnboundBlockCoordinate::new(
                                            -1,
                                            (dy * sn) as UnboundCoordinateType,
                                            (dz * sn) as UnboundCoordinateType,
                                        ),
                                ) {
                                    coords_to_make_dirty.push(coord);
                                }
                            }
                        }
                    }
                    BlockDirection::PosX => {
                        for dz in 0..N_CHECKS {
                            for dy in 0..N_CHECKS {
                                coords_to_make_dirty
                                    .push(negative_most_coord + BlockCoordinate::new(CHUNK_DIMENSIONS * scale, dy * sn, dz * sn));
                            }
                        }
                    }
                    BlockDirection::NegY => {
                        for dz in 0..N_CHECKS {
                            for dx in 0..N_CHECKS {
                                if let Ok(coord) = BlockCoordinate::try_from(
                                    negative_most_coord
                                        + UnboundBlockCoordinate::new(
                                            (dx * sn) as UnboundCoordinateType,
                                            -1,
                                            (dz * sn) as UnboundCoordinateType,
                                        ),
                                ) {
                                    coords_to_make_dirty.push(coord);
                                }
                            }
                        }
                    }
                    BlockDirection::PosY => {
                        for dz in 0..N_CHECKS {
                            for dx in 0..N_CHECKS {
                                coords_to_make_dirty
                                    .push(negative_most_coord + BlockCoordinate::new(dx * sn, CHUNK_DIMENSIONS * scale, dz * sn));
                            }
                        }
                    }
                    BlockDirection::NegZ => {
                        for dy in 0..N_CHECKS {
                            for dx in 0..N_CHECKS {
                                if let Ok(coord) = BlockCoordinate::try_from(
                                    negative_most_coord
                                        + UnboundBlockCoordinate::new(
                                            (dx * sn) as UnboundCoordinateType,
                                            (dy * sn) as UnboundCoordinateType,
                                            -1,
                                        ),
                                ) {
                                    coords_to_make_dirty.push(coord);
                                }
                            }
                        }
                    }
                    BlockDirection::PosZ => {
                        for dy in 0..N_CHECKS {
                            for dx in 0..N_CHECKS {
                                coords_to_make_dirty
                                    .push(negative_most_coord + BlockCoordinate::new(dx * sn, dy * sn, CHUNK_DIMENSIONS * scale));
                            }
                        }
                    }
                }
            }
        }
    };
}

fn find_non_dirty(lod: &Lod, offset: Vec3, to_process: &mut Vec<Vec3>, scale: f32) {
    match lod {
        Lod::None => {
            to_process.push(offset);
        }
        Lod::Children(children) => {
            children.iter().enumerate().for_each(|(i, c)| {
                let s4: f32 = scale / 4.0;

                let offset = match i {
                    0 => offset + Vec3::new(-s4, -s4, -s4),
                    1 => offset + Vec3::new(-s4, -s4, s4),
                    2 => offset + Vec3::new(s4, -s4, s4),
                    3 => offset + Vec3::new(s4, -s4, -s4),
                    4 => offset + Vec3::new(-s4, s4, -s4),
                    5 => offset + Vec3::new(-s4, s4, s4),
                    6 => offset + Vec3::new(s4, s4, s4),
                    7 => offset + Vec3::new(s4, s4, -s4),
                    _ => unreachable!(),
                };

                find_non_dirty(c, offset, to_process, scale / 2.0);
            });
        }
        Lod::Single(_, dirty) => {
            if !*dirty {
                to_process.push(offset);
            }
        }
    };
}

#[derive(Debug)]
struct RenderingLod(Task<(Vec<Vec3>, Vec<(ChunkMesh, Vec3, CoordinateType)>)>);

#[derive(Component, Debug)]
struct RenderedLod {
    scale: CoordinateType,
}

#[derive(Debug, Clone, DerefMut, Deref)]
struct LodRendersToDespawn(Arc<Mutex<(Entity, usize)>>);

#[derive(Debug, Resource, Default, Deref, DerefMut)]
struct MeshesToCompute(VecDeque<(Mesh, Entity, Vec<LodRendersToDespawn>)>);

const MESHES_PER_FRAME: usize = 15;

fn kill_all(to_kill: Vec<LodRendersToDespawn>, commands: &mut Commands) {
    for x in to_kill {
        let mut unlocked = x.lock().expect("Failed lock");
        unlocked.1 -= 1;

        if unlocked.1 == 0
            && let Ok(mut ecmds) = commands.get_entity(unlocked.0)
        {
            ecmds.insert(NeedsDespawned);
        }
    }
}

fn compute_meshes_and_kill_dead_entities(
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    mut meshes_to_compute: ResMut<MeshesToCompute>,
) {
    if meshes_to_compute.is_empty() {
        return;
    }

    let mut to_clean_meshes = VecDeque::with_capacity(meshes_to_compute.0.capacity());

    swap(&mut to_clean_meshes, &mut meshes_to_compute.0);

    for (delayed_mesh, entity, to_kill) in to_clean_meshes {
        if commands.get_entity(entity).is_ok() {
            meshes_to_compute.push_back((delayed_mesh, entity, to_kill));
        } else {
            kill_all(to_kill, &mut commands);
        }
    }

    for _ in 0..MESHES_PER_FRAME {
        let Some((delayed_mesh, entity, to_kill)) = meshes_to_compute.0.pop_front() else {
            break;
        };

        // The entity was verified to exist above
        if let Ok(mut ecmds) = commands.get_entity(entity) {
            ecmds.insert(Mesh3d(meshes.add(delayed_mesh)));
        }

        kill_all(to_kill, &mut commands);
    }
}

fn poll_rendering_lods(
    mut commands: Commands,
    structure_lod_meshes_query: Query<&LodMeshes>,
    transform_query: Query<&Transform>,
    rendered_lod_query: Query<&RenderedLod>,
    mut rendering_lods: ResMut<RenderingLods>,
    mut meshes_to_compute: ResMut<MeshesToCompute>,
    mut event_writer: EventWriter<AddMaterialEvent>,
) {
    let mut todo = Vec::with_capacity(rendering_lods.0.capacity());

    swap(&mut rendering_lods.0, &mut todo);

    for (structure_entity, mut rendering_lod) in todo {
        let Some((to_keep_locations, ent_meshes)) = future::block_on(future::poll_once(&mut rendering_lod.0)) else {
            rendering_lods.0.push((structure_entity, rendering_lod));
            continue;
        };

        let mut structure_meshes_component = LodMeshes::default();
        let mut entities_to_add = Vec::new();

        let old_mesh_entities = structure_lod_meshes_query
            .get(structure_entity)
            .map(|x| x.0.clone())
            .unwrap_or_default();

        // grab entities to kill
        //   insert them into list of Arc<Mutex<(Entity, usize)>> where usize represents a counter
        //   loop through every created lod and assign them the dirty entity where they go (or none)

        // once the new entity's mesh is ready, decrease the counter
        // if the counter is 0, despawn the dirty entity.
        //

        for (lod_mesh, offset, scale) in ent_meshes {
            for mesh_material in lod_mesh.mesh_materials {
                // let s = (CHUNK_DIMENSIONS / 2) as f32 * lod_mesh.scale;

                let ent = commands
                    .spawn((
                        Transform::from_translation(offset),
                        Visibility::default(),
                        // Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                        RenderedLod { scale },
                        DespawnWithStructure,
                    ))
                    .id();

                event_writer.write(AddMaterialEvent {
                    entity: ent,
                    add_material_id: mesh_material.material_id,
                    texture_dimensions_index: mesh_material.texture_dimensions_index,
                    material_type: if scale >= 2 { MaterialType::FarAway } else { MaterialType::Normal },
                });

                entities_to_add.push((ent, offset, scale, mesh_material.mesh));

                structure_meshes_component.0.push(ent);
            }
        }

        let mut to_despawn = Vec::with_capacity(old_mesh_entities.len());

        // Any dirty entities are useless now, so kill them
        for mesh_entity in old_mesh_entities {
            let Ok(transform) = transform_query.get(mesh_entity) else {
                unreachable!();
            };

            let is_clean = to_keep_locations.contains(&transform.translation);
            if is_clean {
                structure_meshes_component.push(mesh_entity);
            } else {
                let Ok(rendered_lod) = rendered_lod_query.get(mesh_entity) else {
                    warn!("Invalid mesh entity {mesh_entity:?}!");
                    commands.entity(mesh_entity).insert(NeedsDespawned);
                    continue;
                };

                to_despawn.push((
                    transform.translation,
                    rendered_lod.scale,
                    LodRendersToDespawn(Arc::new(Mutex::new((mesh_entity, 0)))),
                ));
            }
        }

        let Ok(mut entity_commands) = commands.get_entity(structure_entity) else {
            info!("Failed to get ecmds for planet ;(");
            continue;
        };

        for (entity, offset, scale, mesh) in entities_to_add {
            let mut to_kill = vec![];

            for (other_offset, other_scale, counter) in to_despawn.iter() {
                let diff = (offset - *other_offset).abs();
                let max = diff.x.max(diff.y).max(diff.z);

                if CHUNK_DIMENSIONS * scale + CHUNK_DIMENSIONS * *other_scale < max.floor() as CoordinateType {
                    let counter = counter.clone();

                    counter.0.lock().expect("lock failed").1 += 1;

                    to_kill.push(counter);
                }
            }

            meshes_to_compute.0.push_back((mesh, entity, to_kill));
            entity_commands.add_child(entity);
        }

        entity_commands.insert(structure_meshes_component);

        for (_, _, counter) in to_despawn {
            let locked = counter.lock().expect("failed to lock");
            if locked.1 == 0
                && let Ok(mut ecmds) = commands.get_entity(locked.0)
            {
                ecmds.insert(NeedsDespawned);
            }
        }
    }
}

fn hide_lod(mut query: Query<(&Transform, &ChildOf, &mut Visibility, &RenderedLod)>, structure_query: Query<&Structure>) {
    for (transform, parent, mut vis, rendered_lod) in query.iter_mut() {
        if rendered_lod.scale != 1 {
            continue;
        }

        let structure = structure_query.get(parent.parent()).expect("This should always be a structure");

        let translation = transform.translation;
        if let Ok(bc) = structure.relative_coords_to_local_coords_checked(translation.x, translation.y, translation.z) {
            let chunk_coord = ChunkCoordinate::for_block_coordinate(bc);

            // TODO: check if chunk has a mesh
            if structure.get_chunk_state(chunk_coord) == ChunkState::Loaded {
                *vis = Visibility::Hidden;
            } else {
                *vis = Visibility::Inherited;
            }
        }
    }
}

#[derive(Debug, Resource, Default)]
struct NeedLods(HashSet<Entity>);

fn monitor_lods_needs_rendered_system(lods_needed: Query<Entity, Changed<LodComponent>>, mut should_render_lods: ResMut<NeedLods>) {
    for needs_lod in lods_needed.iter() {
        should_render_lods.0.insert(needs_lod);
    }
}

/// Performance hot spot
fn trigger_lod_render(
    blocks: Res<ReadOnlyRegistry<Block>>,
    materials: Res<ReadOnlyManyToOneRegistry<Block, BlockMaterialMapping>>,
    materials_registry: Res<ReadOnlyRegistry<MaterialDefinition>>,
    meshes_registry: Res<ReadOnlyBlockMeshRegistry>,
    block_textures: Res<ReadOnlyRegistry<BlockTextureIndex>>,
    lighting: Res<ReadOnlyRegistry<BlockLighting>>,
    block_rendering_mode: Res<BlockRenderingModes>,
    lods_query: Query<(&LodComponent, &Structure)>,
    mut rendering_lods: ResMut<RenderingLods>,
    mut lods_needed: ResMut<NeedLods>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    let mut needed = HashSet::new();

    swap(&mut lods_needed.0, &mut needed);

    for entity in needed {
        let Ok((lod, structure)) = lods_query.get(entity) else {
            continue;
        };

        // Don't double-render same lod because that causes many issues. Instead put it back into the queue for when the current one finishes.
        if rendering_lods.iter().any(|r_lod| r_lod.0 == entity) {
            lods_needed.0.insert(entity);
            continue;
        }

        info!("NEW LOD RENDER TRIGGERED FOR {entity:?}");

        let blocks = blocks.clone();
        let block_textures = block_textures.clone();
        let materials = materials.clone();
        let meshes_registry = meshes_registry.clone();
        let materials_registry = materials_registry.clone();
        let lighting = lighting.clone();
        // TODO: This one is expensive - make it less expensive.
        let block_rendering_modes = block_rendering_mode.clone();

        let chunk_dimensions = structure.chunk_dimensions().x;
        let block_dimensions = structure.block_dimensions().x;

        let lod = lod.clone();

        let task = thread_pool.spawn(async move {
            let mut lod = lod.0.lock().unwrap();
            let mut non_dirty = vec![];

            mark_adjacent_chunks_dirty(&mut lod, chunk_dimensions);

            find_non_dirty(&lod, Vec3::ZERO, &mut non_dirty, block_dimensions as f32);

            // by making the Vec an Option<Vec> I can take ownership of it later, which I cannot do with
            // just a plain Mutex<Vec>.
            // https://stackoverflow.com/questions/30573188/cannot-move-data-out-of-a-mutex
            let to_process: Mutex<Option<Vec<(ChunkMesh, Vec3, CoordinateType)>>> = Mutex::new(Some(Vec::new()));

            let blocks = blocks.registry();
            let block_textures = block_textures.registry();
            let materials = materials.registry();
            let meshes_registry: std::sync::RwLockReadGuard<'_, ManyToOneRegistry<Block, crate::rendering::BlockMeshInformation>> =
                meshes_registry.registry();
            let materials_registry = materials_registry.registry();
            let lighting = lighting.registry();

            recursively_process_lod(
                &lod,
                chunk_dimensions,
                chunk_dimensions,
                &lod,
                BlockCoordinate::ZERO,
                Vec3::ZERO,
                &to_process,
                &blocks,
                &materials,
                &meshes_registry,
                &block_textures,
                &materials_registry,
                &lighting,
                &block_rendering_modes,
            );

            mark_non_dirty(&mut lod);

            let to_process_chunks = to_process.lock().unwrap().take().unwrap();

            (non_dirty, to_process_chunks)
        });

        rendering_lods.push((entity, RenderingLod(task)));
    }
}

fn mark_non_dirty(lod: &mut Lod) {
    match lod {
        Lod::None => {}
        Lod::Single(_, dirty) => *dirty = false,
        Lod::Children(children) => children.iter_mut().for_each(mark_non_dirty),
    }
}

#[derive(Resource, Debug, Default, Deref, DerefMut)]
struct RenderingLods(Vec<(Entity, RenderingLod)>);

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            monitor_lods_needs_rendered_system,
            trigger_lod_render,
            poll_rendering_lods,
            hide_lod,
            compute_meshes_and_kill_dead_entities,
        )
            .chain()
            .in_set(MaterialsSystemSet::RequestMaterialChanges)
            .run_if(in_state(GameState::Playing)),
    );

    add_statebound_resource::<RenderingLods, GameState>(app, GameState::Playing);
    add_statebound_resource::<NeedLods, GameState>(app, GameState::Playing);
    add_statebound_resource::<MeshesToCompute, GameState>(app, GameState::Playing);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_make_dirty() {
        let mut lod = Lod::Children(Box::new([
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), true),
            Lod::Single(Default::default(), false),
        ]));

        // 5 6 7 2

        mark_adjacent_chunks_dirty(&mut lod, 2);

        match lod {
            Lod::Children(c) => {
                assert!(
                    matches!(
                        c.as_ref(),
                        [
                            Lod::Single(_, false),
                            Lod::Single(_, false),
                            Lod::Single(_, true),
                            Lod::Single(_, false),
                            Lod::Single(_, false),
                            Lod::Single(_, true),
                            Lod::Single(_, true),
                            Lod::Single(_, true),
                        ]
                    ),
                    "{c:?}"
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_make_dirty_2() {
        let mut lod = Lod::Children(Box::new([
            Lod::Single(Default::default(), true),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
        ]));

        mark_adjacent_chunks_dirty(&mut lod, 2);

        match lod {
            Lod::Children(c) => {
                assert!(
                    matches!(
                        c.as_ref(),
                        [
                            Lod::Single(_, true),
                            Lod::Single(_, true),
                            Lod::Single(_, false),
                            Lod::Single(_, true),
                            Lod::Single(_, true),
                            Lod::Single(_, false),
                            Lod::Single(_, false),
                            Lod::Single(_, false),
                        ]
                    ),
                    "{c:?}"
                );
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_make_dirty_sub_child() {
        let mut lod = Lod::Children(Box::new([
            Lod::Children(Box::new([
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), true),
            ])),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
        ]));

        mark_adjacent_chunks_dirty(&mut lod, 4);

        match lod {
            Lod::Children(c) => match c.as_ref() {
                [
                    Lod::Children(c),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, true),
                    Lod::Single(_, true),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                ] => match c.as_ref() {
                    [
                        Lod::Single(_, false),
                        Lod::Single(_, false),
                        Lod::Single(_, false),
                        Lod::Single(_, true),
                        Lod::Single(_, true),
                        Lod::Single(_, false),
                        Lod::Single(_, true),
                        Lod::Single(_, true),
                    ] => {}
                    _ => panic!("{c:?}"),
                },
                _ => panic!("{c:?}"),
            },
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_make_dirty_sub_child_2() {
        let mut lod = Lod::Children(Box::new([
            Lod::Children(Box::new([
                Lod::Children(Box::new([
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), false),
                    Lod::Single(Default::default(), true),
                ])),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
                Lod::Single(Default::default(), false),
            ])),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
            Lod::Single(Default::default(), false),
        ]));

        mark_adjacent_chunks_dirty(&mut lod, 8);

        match lod {
            Lod::Children(c) => match c.as_ref() {
                [
                    Lod::Children(c),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                    Lod::Single(_, false),
                ] => match c.as_ref() {
                    [
                        Lod::Children(c),
                        Lod::Single(_, false),
                        Lod::Single(_, false),
                        Lod::Single(_, true),
                        Lod::Single(_, true),
                        Lod::Single(_, false),
                        Lod::Single(_, false),
                        Lod::Single(_, false),
                    ] => match c.as_ref() {
                        [
                            Lod::Single(_, false),
                            Lod::Single(_, false),
                            Lod::Single(_, false),
                            Lod::Single(_, true),
                            Lod::Single(_, true),
                            Lod::Single(_, false),
                            Lod::Single(_, true),
                            Lod::Single(_, true),
                        ] => {}
                        _ => panic!("{c:?}"),
                    },
                    _ => panic!("{c:?}"),
                },
                _ => panic!("{c:?}"),
            },
            _ => unreachable!(),
        }
    }
}
