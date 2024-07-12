//! Responsible for rendering planet LODs.
//!
//! The code in this file is very smelly.
//!
//! I'm sorry. I'll fix it when I feel inspired.

use std::{
    collections::VecDeque,
    mem::swap,
    sync::{Arc, Mutex},
};

use bevy::{
    prelude::*,
    render::{
        mesh::{MeshVertexAttribute, VertexAttributeValues},
        primitives::Aabb,
    },
    tasks::{AsyncComputeTaskPool, Task},
    utils::{hashbrown::HashMap, HashSet},
};
use futures_lite::future;

use cosmos_core::{
    block::{Block, BlockFace},
    ecs::NeedsDespawned,
    registry::{
        identifiable::Identifiable,
        many_to_one::{ManyToOneRegistry, ReadOnlyManyToOneRegistry},
        ReadOnlyRegistry, Registry,
    },
    structure::{
        block_storage::BlockStorer,
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF},
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        lod::{Lod, LodComponent},
        lod_chunk::LodChunk,
        shared::DespawnWithStructure,
        ChunkState, Structure,
    },
    utils::array_utils::expand,
};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    asset::{
        asset_loading::{BlockNeighbors, BlockTextureIndex},
        materials::{add_materials, remove_materials, AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType},
    },
    ecs::add_statebound_resource,
    state::game_state::GameState,
};

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation, ReadOnlyBlockMeshRegistry};

#[derive(Debug)]
struct MeshMaterial {
    mesh: Mesh,
    material_id: u16,
}

#[derive(Debug)]
struct LodMesh {
    mesh_materials: Vec<MeshMaterial>,
    scale: f32,
}

#[derive(Default, Debug)]
struct MeshInfo {
    mesh_builder: CosmosMeshBuilder,
}

impl MeshBuilder for MeshInfo {
    #[inline]
    fn add_mesh_information(
        &mut self,
        mesh_info: &MeshInformation,
        position: Vec3,
        uvs: Rect,
        texture_index: u32,
        additional_info: Vec<(MeshVertexAttribute, VertexAttributeValues)>,
    ) {
        self.mesh_builder
            .add_mesh_information(mesh_info, position, uvs, texture_index, additional_info);
    }

    fn build_mesh(self) -> Mesh {
        self.mesh_builder.build_mesh()
    }
}

#[derive(Default, Debug)]
struct ChunkRenderer {
    meshes: HashMap<u16, MeshInfo>,
    scale: f32,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    pub fn render(
        &mut self,
        scale: f32,
        materials: &ManyToOneRegistry<Block, BlockMaterialMapping>,
        materials_registry: &Registry<MaterialDefinition>,
        lod: &LodChunk,
        left: Option<&LodChunk>,
        right: Option<&LodChunk>,
        bottom: Option<&LodChunk>,
        top: Option<&LodChunk>,
        back: Option<&LodChunk>,
        front: Option<&LodChunk>,
        blocks: &Registry<Block>,
        meshes: &BlockMeshRegistry,
        block_textures: &Registry<BlockTextureIndex>,
    ) {
        self.scale = scale;

        let cd2 = CHUNK_DIMENSIONSF / 2.0;

        let mut faces = Vec::with_capacity(6);

        for (coords, (block_id, block_info)) in lod
            .blocks()
            .copied()
            .zip(lod.block_info_iterator().copied())
            .enumerate()
            .map(|(i, block)| {
                (
                    ChunkBlockCoordinate::from(expand(i, CHUNK_DIMENSIONS as usize, CHUNK_DIMENSIONS as usize)),
                    block,
                )
            })
            .filter(|(coords, _)| lod.has_block_at(*coords))
        {
            let (center_offset_x, center_offset_y, center_offset_z) = (
                coords.x as f32 - cd2 + 0.5,
                coords.y as f32 - cd2 + 0.5,
                coords.z as f32 - cd2 + 0.5,
            );
            let actual_block = blocks.from_numeric_id(block_id);

            #[inline(always)]
            fn check(c: &LodChunk, block: u16, actual_block: &Block, blocks: &Registry<Block>, coords: ChunkBlockCoordinate) -> bool {
                (block != c.block_at(coords) || !actual_block.is_full()) && c.has_see_through_block_at(coords, blocks)
            }

            let (x, y, z) = (coords.x, coords.y, coords.z);

            // right
            if (x != CHUNK_DIMENSIONS - 1 && check(lod, block_id, actual_block, blocks, coords.right()))
                || (x == CHUNK_DIMENSIONS - 1
                    && (right
                        .map(|c| check(c, block_id, actual_block, blocks, ChunkBlockCoordinate::new(0, y, z)))
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Left);
            }
            // left
            if (x != 0
                && check(
                    lod,
                    block_id,
                    actual_block,
                    blocks,
                    coords.left().expect("Checked in first condition"),
                ))
                || (x == 0
                    && (left
                        .map(|c| {
                            check(
                                c,
                                block_id,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, y, z),
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Right);
            }

            // top
            if (y != CHUNK_DIMENSIONS - 1 && check(lod, block_id, actual_block, blocks, coords.top()))
                || (y == CHUNK_DIMENSIONS - 1
                    && top
                        .map(|c| check(c, block_id, actual_block, blocks, ChunkBlockCoordinate::new(x, 0, z)))
                        .unwrap_or(true))
            {
                faces.push(BlockFace::Top);
            }
            // bottom
            if (y != 0
                && check(
                    lod,
                    block_id,
                    actual_block,
                    blocks,
                    coords.bottom().expect("Checked in first condition"),
                ))
                || (y == 0
                    && (bottom
                        .map(|c| {
                            check(
                                c,
                                block_id,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, CHUNK_DIMENSIONS - 1, z),
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Bottom);
            }

            // front
            if (z != CHUNK_DIMENSIONS - 1 && check(lod, block_id, actual_block, blocks, coords.front()))
                || (z == CHUNK_DIMENSIONS - 1
                    && (front
                        .map(|c| check(c, block_id, actual_block, blocks, ChunkBlockCoordinate::new(x, y, 0)))
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Front);
            }
            // back
            if (z != 0
                && check(
                    lod,
                    block_id,
                    actual_block,
                    blocks,
                    coords.back().expect("Checked in first condition"),
                ))
                || (z == 0
                    && (back
                        .map(|c| {
                            check(
                                c,
                                block_id,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, y, CHUNK_DIMENSIONS - 1),
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Back);
            }

            if !faces.is_empty() {
                let block = blocks.from_numeric_id(block_id);

                let Some(block_material_mapping) = materials.get_value(block) else {
                    continue;
                };

                let mat_id = block_material_mapping.material_id();

                let material_definition = materials_registry.from_numeric_id(mat_id);

                let Some(mesh) = meshes.get_value(block) else {
                    continue;
                };

                if !self.meshes.contains_key(&mat_id) {
                    self.meshes.insert(mat_id, Default::default());
                }

                let mesh_builder = self.meshes.get_mut(&mat_id).unwrap();

                let block_rotation = block_info.get_rotation();

                let rotation = block_rotation.as_quat();

                for face in faces.iter().map(|face| block_rotation.rotate_face(*face)) {
                    let index = block_textures
                        .from_id(block.unlocalized_name())
                        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                    let neighbors = BlockNeighbors::empty();

                    let maybe_img_idx = if self.scale > 8.0 {
                        index
                            .atlas_index_for_lod(neighbors)
                            .map(Some)
                            .unwrap_or_else(|| index.atlas_index_from_face(face, neighbors))
                    } else {
                        index.atlas_index_from_face(face, neighbors)
                    };

                    let Some(image_index) = maybe_img_idx else {
                        warn!("Missing image index for face {face} -- {index:?}");
                        continue;
                    };

                    let mut one_mesh_only = false;

                    let mut mesh_info = mesh
                        .info_for_face(face, false)
                        .unwrap_or_else(|| {
                            one_mesh_only = true;

                            mesh.info_for_whole_block()
                                .expect("Block must have either face or whole block meshes")
                        })
                        .clone();

                    for pos in mesh_info.positions.iter_mut() {
                        *pos = rotation.mul_vec3(Vec3::new(pos[0] * scale, pos[1] * scale, pos[2] * scale)).into();
                    }

                    for norm in mesh_info.normals.iter_mut() {
                        *norm = rotation.mul_vec3((*norm).into()).into();
                    }

                    mesh_builder.add_mesh_information(
                        &mesh_info,
                        Vec3::new(center_offset_x * scale, center_offset_y * scale, center_offset_z * scale),
                        Rect::new(0.0, 0.0, 1.0, 1.0),
                        image_index,
                        material_definition.add_material_data(block_id, &mesh_info),
                    );

                    if one_mesh_only {
                        break;
                    }
                }

                faces.clear();
            }
        }
    }

    fn create_mesh(self) -> LodMesh {
        let mut mesh_materials = Vec::new();

        for (material, chunk_mesh_info) in self.meshes {
            let mesh = chunk_mesh_info.build_mesh();

            mesh_materials.push(MeshMaterial {
                material_id: material,
                mesh,
            });
        }

        LodMesh {
            mesh_materials,
            scale: self.scale,
        }
    }
}

#[derive(Component, Debug, Reflect, Default, Deref, DerefMut)]
struct LodMeshes(Vec<Entity>);

fn recursively_process_lod(
    lod_path: LodPath,
    to_process: &Mutex<Option<Vec<(LodMesh, Vec3, CoordinateType)>>>,
    blocks: &Registry<Block>,
    materials: &ManyToOneRegistry<Block, BlockMaterialMapping>,
    meshes_registry: &BlockMeshRegistry,
    block_textures: &Registry<BlockTextureIndex>,
    materials_registry: &Registry<MaterialDefinition>,
) {
    let (lod_path_info, _) = match &lod_path {
        LodPath::Top(lod_path_info) => (lod_path_info, None),
        LodPath::HasParent(lod_path_info, parent) => (lod_path_info, Some(parent)),
    };

    match lod_path_info.lod {
        Lod::None => {}
        Lod::Children(children) => {
            children.par_iter().enumerate().for_each(|(i, c)| {
                let s4 = lod_path_info.scale / 4.0;

                let offset = lod_path_info.offset
                    + match i {
                        0 => Vec3::new(-s4, -s4, -s4),
                        1 => Vec3::new(-s4, -s4, s4),
                        2 => Vec3::new(s4, -s4, s4),
                        3 => Vec3::new(s4, -s4, -s4),
                        4 => Vec3::new(-s4, s4, -s4),
                        5 => Vec3::new(-s4, s4, s4),
                        6 => Vec3::new(s4, s4, s4),
                        7 => Vec3::new(s4, s4, -s4),
                        _ => unreachable!(),
                    };

                recursively_process_lod(
                    LodPath::HasParent(
                        PathInfo {
                            lod: c,
                            depth: lod_path_info.depth + 1,
                            scale: lod_path_info.scale / 2.0,
                            offset,
                        },
                        &lod_path,
                    ),
                    to_process,
                    blocks,
                    materials,
                    meshes_registry,
                    block_textures,
                    materials_registry,
                );
            });
        }
        Lod::Single(lod_chunk, dirty) => {
            if !*dirty {
                return;
            }

            let mut renderer = ChunkRenderer::new();

            let mut neighbors = [None; 6];

            lod_path.find_neighbors(lod_path_info, &mut neighbors);

            renderer.render(
                lod_path_info.scale,
                materials,
                materials_registry,
                lod_chunk,
                neighbors[0],
                neighbors[1],
                neighbors[2],
                neighbors[3],
                neighbors[4],
                neighbors[5],
                blocks,
                meshes_registry,
                block_textures,
            );

            let mut mutex = to_process.lock().expect("Error locking to_process vec!");

            mutex.as_mut().unwrap().push((
                renderer.create_mesh(),
                Vec3::new(
                    lod_path_info.offset.x * CHUNK_DIMENSIONSF,
                    lod_path_info.offset.y * CHUNK_DIMENSIONSF,
                    lod_path_info.offset.z * CHUNK_DIMENSIONSF,
                ),
                lod_path_info.scale as CoordinateType,
            ));
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
struct RenderingLod(Task<(Vec<Vec3>, Vec<(LodMesh, Vec3, CoordinateType)>)>);

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

        if unlocked.1 == 0 {
            if let Some(mut ecmds) = commands.get_entity(unlocked.0) {
                ecmds.insert(NeedsDespawned);
            }
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
        if commands.get_entity(entity).is_some() {
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
        if let Some(mut ecmds) = commands.get_entity(entity) {
            ecmds.insert(meshes.add(delayed_mesh));
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
        if let Some((to_keep_locations, ent_meshes)) = future::block_on(future::poll_once(&mut rendering_lod.0)) {
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

            for (lod_mesh, offset, scale) in ent_meshes {
                for mesh_material in lod_mesh.mesh_materials {
                    let s = (CHUNK_DIMENSIONS / 2) as f32 * lod_mesh.scale;

                    let ent = commands
                        .spawn((
                            TransformBundle::from_transform(Transform::from_translation(offset)),
                            VisibilityBundle::default(),
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ ~0.13~ 0.never is released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                            RenderedLod { scale },
                            DespawnWithStructure,
                        ))
                        .id();

                    event_writer.send(AddMaterialEvent {
                        entity: ent,
                        add_material_id: mesh_material.material_id,
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

                let is_clean = to_keep_locations.iter().any(|&x| x == transform.translation);
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

            let Some(mut entity_commands) = commands.get_entity(structure_entity) else {
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

            // if let Ok(mut l) = lod_query.get_mut(structure_entity) {
            //     // Avoid recursively re-rendering the lod. The only thing changing about the lod are the dirty flags.
            //     // This could be refactored to store dirty flags elsewhere, but I'm not sure about the performance cost of that.
            //     // *(l.bypass_change_detection()) = lod;
            //     *l.0.lock().unwrap() = lod;
            // } else {
            //     entity_commands.insert(LodComponent(Arc::new(Mutex::new(lod))));
            // }

            entity_commands.insert(structure_meshes_component);

            for (_, _, counter) in to_despawn {
                let locked = counter.lock().expect("failed to lock");
                if locked.1 == 0 {
                    if let Some(mut ecmds) = commands.get_entity(locked.0) {
                        ecmds.insert(NeedsDespawned);
                    }
                }
            }
        } else {
            rendering_lods.0.push((structure_entity, rendering_lod))
        }
    }
}

fn hide_lod(mut query: Query<(&Transform, &Parent, &mut Visibility, &RenderedLod)>, structure_query: Query<&Structure>) {
    for (transform, parent, mut vis, rendered_lod) in query.iter_mut() {
        if rendered_lod.scale != 1 {
            continue;
        }

        let structure = structure_query.get(parent.get()).expect("This should always be a structure");

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

struct PathInfo<'a> {
    lod: &'a Lod,
    depth: usize,
    scale: f32,
    offset: Vec3,
}

enum LodPath<'a> {
    Top(PathInfo<'a>),
    HasParent(PathInfo<'a>, &'a LodPath<'a>),
}

/// Checks if b is within or directly next to a
#[must_use]
fn check_within_or_next_to(a: (Vec3, f32), b: (Vec3, f32)) -> bool {
    let (a_off, a_scale) = a;
    let (b_off, b_scale) = b;
    let s2 = a_scale / 2.0;

    let a_min = a_off - Vec3::splat(s2);
    let a_max = a_off + Vec3::splat(s2);

    let s2 = b_scale / 2.0;

    let b_min = b_off - Vec3::splat(s2);
    let b_max = b_off + Vec3::splat(s2);

    b_max.x >= a_min.x && b_max.y >= a_min.y && b_max.z >= a_min.z && b_min.x <= a_max.x && b_min.y <= a_max.y && b_min.z <= a_max.z
}

fn we_need_to_go_deeper<'a>(
    offset: Vec3,
    scale: f32,
    lod: &'a Lod,
    depth: usize,
    searching_for_path_info: &PathInfo,
    neighbors: &mut [Option<&'a LodChunk>; 6],
) {
    if !neighbors.iter().any(|x| x.is_none()) {
        // Neighbors have already been found, stop looking for more
        return;
    }

    let bounds = (searching_for_path_info.offset, searching_for_path_info.scale);

    if !check_within_or_next_to((offset, scale), bounds) {
        return;
    }

    if let Lod::Children(children) = lod {
        for (i, child_lod) in children.iter().enumerate() {
            // if i == index {
            //     // This will check against every single index, not just the ones that are a part of the same lod group as the index passed in.
            //     // However, because a neighbor will never share the same index as the one we're checking, this check is perfectly fine.
            //     continue;
            // }

            let s4 = scale / 4.0;

            let new_offset = offset
                + match i {
                    0 => Vec3::new(-s4, -s4, -s4),
                    1 => Vec3::new(-s4, -s4, s4),
                    2 => Vec3::new(s4, -s4, s4),
                    3 => Vec3::new(s4, -s4, -s4),
                    4 => Vec3::new(-s4, s4, -s4),
                    5 => Vec3::new(-s4, s4, s4),
                    6 => Vec3::new(s4, s4, s4),
                    7 => Vec3::new(s4, s4, -s4),
                    _ => unreachable!(),
                };

            match child_lod {
                Lod::Single(chunk, _) => {
                    if searching_for_path_info.depth == depth + 1 && check_within_or_next_to((new_offset, scale / 2.0), bounds) {
                        let diff = new_offset - searching_for_path_info.offset;
                        if diff.y == 0.0 && diff.z == 0.0 {
                            if diff.x < 0.0 {
                                neighbors[0] = Some(chunk);
                            } else if diff.x > 0.0 {
                                neighbors[1] = Some(chunk);
                            }
                        } else if diff.x == 0.0 && diff.z == 0.0 {
                            if diff.y < 0.0 {
                                neighbors[2] = Some(chunk);
                            } else if diff.y > 0.0 {
                                neighbors[3] = Some(chunk);
                            }
                        } else if diff.x == 0.0 && diff.y == 0.0 {
                            if diff.z < 0.0 {
                                neighbors[4] = Some(chunk);
                            } else if diff.z > 0.0 {
                                neighbors[5] = Some(chunk);
                            }
                        }
                    }
                }
                Lod::Children(_) => we_need_to_go_deeper(new_offset, scale / 2.0, child_lod, depth + 1, searching_for_path_info, neighbors),
                Lod::None => {}
            }
        }
    }
}

impl<'a> LodPath<'a> {
    /// Order: left, right, bottom, top, back, front
    fn find_neighbors(&self, searching_for_path_info: &PathInfo, neighbors: &mut [Option<&'a LodChunk>; 6]) {
        match self {
            LodPath::Top(path_info) => we_need_to_go_deeper(
                path_info.offset,
                path_info.scale,
                path_info.lod,
                path_info.depth,
                searching_for_path_info,
                neighbors,
            ),
            LodPath::HasParent(_, parent) => parent.find_neighbors(searching_for_path_info, neighbors),
        }
    }
}

/// Performance hot spot
fn trigger_lod_render(
    blocks: Res<ReadOnlyRegistry<Block>>,
    materials: Res<ReadOnlyManyToOneRegistry<Block, BlockMaterialMapping>>,
    materials_registry: Res<ReadOnlyRegistry<MaterialDefinition>>,
    meshes_registry: Res<ReadOnlyBlockMeshRegistry>,
    block_textures: Res<ReadOnlyRegistry<BlockTextureIndex>>,
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

        let chunk_dimensions = structure.chunk_dimensions().x;
        let block_dimensions = structure.block_dimensions().x;

        let lod = lod.clone();

        let task = thread_pool.spawn(async move {
            let mut lod = lod.0.lock().unwrap();
            let mut non_dirty = vec![];
            find_non_dirty(&lod, Vec3::ZERO, &mut non_dirty, block_dimensions as f32);

            // by making the Vec an Option<Vec> I can take ownership of it later, which I cannot do with
            // just a plain Mutex<Vec>.
            // https://stackoverflow.com/questions/30573188/cannot-move-data-out-of-a-mutex
            let to_process: Mutex<Option<Vec<(LodMesh, Vec3, CoordinateType)>>> = Mutex::new(Some(Vec::new()));

            let blocks = blocks.registry();
            let block_textures = block_textures.registry();
            let materials = materials.registry();
            let meshes_registry: std::sync::RwLockReadGuard<'_, ManyToOneRegistry<Block, crate::rendering::BlockMeshInformation>> =
                meshes_registry.registry();
            let materials_registry = materials_registry.registry();

            // let mut cloned_lod = lod.clone();

            let lod_path = LodPath::Top(PathInfo {
                lod: &lod,
                depth: 1,
                scale: chunk_dimensions as f32,
                offset: Vec3::ZERO,
            });
            recursively_process_lod(
                lod_path,
                &to_process,
                &blocks,
                &materials,
                &meshes_registry,
                &block_textures,
                &materials_registry,
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
            .before(remove_materials)
            .before(add_materials)
            .run_if(in_state(GameState::Playing)),
    );

    add_statebound_resource::<RenderingLods>(app, GameState::Playing);
    add_statebound_resource::<NeedLods>(app, GameState::Playing);
    add_statebound_resource::<MeshesToCompute>(app, GameState::Playing);
}
