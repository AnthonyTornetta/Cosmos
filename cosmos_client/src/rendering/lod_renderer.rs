use std::{
    collections::VecDeque,
    f32::consts::PI,
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
        lod::{Lod, ReadOnlyLod},
        lod_chunk::LodChunk,
        ChunkState, Structure,
    },
    utils::array_utils::expand,
};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    asset::{
        asset_loading::BlockTextureIndex,
        materials::{add_materials, remove_materials, AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType},
    },
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
    fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    fn render(
        &mut self,
        scale: f32,
        offset: Vec3,
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
                faces.push(BlockFace::Right);
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
                faces.push(BlockFace::Left);
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
                faces.push(BlockFace::Back);
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
                faces.push(BlockFace::Front);
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

                let rotation = block_info.get_rotation();

                for face in faces.iter().map(|x| BlockFace::rotate_face(*x, rotation)) {
                    let index = block_textures
                        .from_id(block.unlocalized_name())
                        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                    let maybe_img_idx = if self.scale > 8.0 {
                        index
                            .atlas_index("lod")
                            .map(Some)
                            .unwrap_or_else(|| index.atlas_index_from_face(face))
                    } else {
                        index.atlas_index_from_face(face)
                    };

                    let Some(image_index) = maybe_img_idx else {
                        warn!("Missing image index -- {index:?}");
                        continue;
                    };

                    let rotation = match rotation {
                        BlockFace::Top => Quat::IDENTITY,
                        BlockFace::Front => Quat::from_axis_angle(Vec3::X, PI / 2.0),
                        BlockFace::Back => Quat::from_axis_angle(Vec3::X, -PI / 2.0),
                        BlockFace::Left => Quat::from_axis_angle(Vec3::Z, PI / 2.0),
                        BlockFace::Right => Quat::from_axis_angle(Vec3::Z, -PI / 2.0),
                        BlockFace::Bottom => Quat::from_axis_angle(Vec3::X, PI),
                    };

                    let mut one_mesh_only = false;

                    let mut mesh_info = mesh
                        .info_for_face(face)
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
                        offset * CHUNK_DIMENSIONSF + Vec3::new(center_offset_x * scale, center_offset_y * scale, center_offset_z * scale),
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
    lod: &mut Lod,
    offset: Vec3,
    to_process: &Mutex<Option<Vec<(LodMesh, Vec3, CoordinateType)>>>,
    blocks: &Registry<Block>,
    materials: &ManyToOneRegistry<Block, BlockMaterialMapping>,
    meshes_registry: &BlockMeshRegistry,
    block_textures: &Registry<BlockTextureIndex>,
    materials_registry: &Registry<MaterialDefinition>,
    scale: f32,
) {
    match lod {
        Lod::None => {}
        Lod::Children(children) => {
            children.par_iter_mut().enumerate().for_each(|(i, c)| {
                let s4 = scale / 4.0;

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

                recursively_process_lod(
                    c,
                    offset,
                    to_process,
                    blocks,
                    materials,
                    meshes_registry,
                    block_textures,
                    materials_registry,
                    scale / 2.0,
                );
            });
        }
        Lod::Single(lod_chunk, dirty) => {
            if !*dirty {
                return;
            }

            *dirty = false;

            let mut renderer = ChunkRenderer::new();

            renderer.render(
                scale,
                Vec3::ZERO,
                materials,
                materials_registry,
                lod_chunk,
                None,
                None,
                None,
                None,
                None,
                None,
                blocks,
                meshes_registry,
                block_textures,
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
struct RenderingLod(Task<(Vec<Vec3>, Vec<(LodMesh, Vec3, CoordinateType)>, Lod)>);

#[derive(Component, Debug)]
struct RenderedLod {
    scale: CoordinateType,
}

#[derive(Debug, Clone, DerefMut, Deref)]
struct ToKill(Arc<Mutex<(Entity, usize)>>);

#[derive(Debug, Resource, Default, Deref, DerefMut)]
struct MeshesToCompute(VecDeque<(Mesh, Entity, Vec<ToKill>)>);

const MESHES_PER_FRAME: usize = 15;

fn kill_all(to_kill: Vec<ToKill>, commands: &mut Commands) {
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
    // bypass change detection to not trigger re-render
    mut lod_query: Query<&mut Lod>,

    mut meshes_to_compute: ResMut<MeshesToCompute>,
    mut event_writer: EventWriter<AddMaterialEvent>,
) {
    let mut todo = Vec::with_capacity(rendering_lods.0.capacity());

    swap(&mut rendering_lods.0, &mut todo);

    for (structure_entity, mut rendering_lod) in todo {
        if let Some((to_keep_locations, ent_meshes, lod)) = future::block_on(future::poll_once(&mut rendering_lod.0)) {
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
                            Visibility::default(),
                            ComputedVisibility::default(),
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                            RenderedLod { scale },
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
                        ToKill(Arc::new(Mutex::new((mesh_entity, 0)))),
                    ));
                }
            }

            let mut entity_commands = commands.entity(structure_entity);

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

            if let Ok(mut l) = lod_query.get_mut(structure_entity) {
                // Avoid recursively re-rendering the lod. The only thing changing about the lod are the dirty flags.
                // This could be refactored to store dirty flags elsewhere, but I'm not sure about the performance cost of that.
                *(l.bypass_change_detection()) = lod;
            } else {
                entity_commands.insert(lod);
            }

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

fn monitor_lods_needs_rendered_system(lods_needed: Query<Entity, Changed<Lod>>, mut should_render_lods: ResMut<NeedLods>) {
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
    lods_query: Query<(&ReadOnlyLod, &Structure)>,
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
            let lod = lod.inner();
            let mut non_dirty = vec![];
            find_non_dirty(&lod, Vec3::ZERO, &mut non_dirty, block_dimensions as f32);

            // by making the Vec an Option<Vec> I can take ownership of it later, which I cannot do with
            // just a plain Mutex<Vec>.
            // https://stackoverflow.com/questions/30573188/cannot-move-data-out-of-a-mutex
            let to_process: Mutex<Option<Vec<(LodMesh, Vec3, CoordinateType)>>> = Mutex::new(Some(Vec::new()));

            let blocks = blocks.registry();
            let block_textures = block_textures.registry();
            let materials = materials.registry();
            let meshes_registry = meshes_registry.registry();
            let materials_registry = materials_registry.registry();

            let mut cloned_lod = lod.clone();

            recursively_process_lod(
                &mut cloned_lod,
                Vec3::ZERO,
                &to_process,
                &blocks,
                &materials,
                &meshes_registry,
                &block_textures,
                &materials_registry,
                chunk_dimensions as f32,
            );

            let to_process_chunks = to_process.lock().unwrap().take().unwrap();

            (non_dirty, to_process_chunks, cloned_lod)
        });

        rendering_lods.push((entity, RenderingLod(task)));
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
    )
    .insert_resource(RenderingLods::default())
    .insert_resource(NeedLods::default())
    .init_resource::<MeshesToCompute>();
}
