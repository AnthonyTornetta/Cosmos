use std::{f32::consts::PI, sync::Mutex};

use bevy::{pbr::NotShadowCaster, prelude::*, render::primitives::Aabb, utils::HashMap};
use cosmos_core::{
    block::{Block, BlockFace},
    registry::{identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
    structure::{
        block_storage::BlockStorer,
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF},
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
        lod::Lod,
        lod_chunk::LodChunk,
        Structure,
    },
    utils::array_utils::expand,
};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    asset::asset_loading::{BlockTextureIndex, MainAtlas},
    materials::CosmosMaterial,
    state::game_state::GameState,
};

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation};

#[derive(Debug)]
struct MeshMaterial {
    mesh: Mesh,
    material: Handle<StandardMaterial>,
}

#[derive(Debug)]
struct LodMesh {
    mesh_materials: Vec<MeshMaterial>,
    scale: f32,
}

#[derive(Default, Debug, Reflect)]
struct ChunkRendererInstance {
    indices: Vec<u32>,
    uvs: Vec<[f32; 2]>,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
}

#[derive(Default, Debug, Reflect)]
struct MeshInfo {
    renderer: ChunkRendererInstance,
    mesh_builder: CosmosMeshBuilder,
}

impl MeshBuilder for MeshInfo {
    #[inline]
    fn add_mesh_information(&mut self, mesh_info: &MeshInformation, position: Vec3, uvs: Rect) {
        self.mesh_builder.add_mesh_information(mesh_info, position, uvs);
    }

    fn build_mesh(self) -> Mesh {
        self.mesh_builder.build_mesh()
    }
}

#[derive(Default, Debug, Reflect)]
struct ChunkRenderer {
    meshes: HashMap<Handle<StandardMaterial>, MeshInfo>,
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
        atlas: &MainAtlas,
        materials: &ManyToOneRegistry<Block, CosmosMaterial>,
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

        for (coords, (block, block_info)) in lod
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
            let actual_block = blocks.from_numeric_id(block);

            #[inline(always)]
            fn check(c: &LodChunk, block: u16, actual_block: &Block, blocks: &Registry<Block>, coords: ChunkBlockCoordinate) -> bool {
                (block != c.block_at(coords) || !actual_block.is_full()) && c.has_see_through_block_at(coords, blocks)
            }

            let (x, y, z) = (coords.x, coords.y, coords.z);

            // right
            if (x != CHUNK_DIMENSIONS - 1 && check(lod, block, actual_block, blocks, coords.right()))
                || (x == CHUNK_DIMENSIONS - 1
                    && (right
                        .map(|c| check(c, block, actual_block, blocks, ChunkBlockCoordinate::new(0, y, z)))
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Right);
            }
            // left
            if (x != 0 && check(lod, block, actual_block, blocks, coords.left().expect("Checked in first condition")))
                || (x == 0
                    && (left
                        .map(|c| {
                            check(
                                c,
                                block,
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
            if (y != CHUNK_DIMENSIONS - 1 && check(lod, block, actual_block, blocks, coords.top()))
                || (y == CHUNK_DIMENSIONS - 1
                    && top
                        .map(|c| check(c, block, actual_block, blocks, ChunkBlockCoordinate::new(x, 0, z)))
                        .unwrap_or(true))
            {
                faces.push(BlockFace::Top);
            }
            // bottom
            if (y != 0
                && check(
                    lod,
                    block,
                    actual_block,
                    blocks,
                    coords.bottom().expect("Checked in first condition"),
                ))
                || (y == 0
                    && (bottom
                        .map(|c| {
                            check(
                                c,
                                block,
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
            if (z != CHUNK_DIMENSIONS - 1 && check(lod, block, actual_block, blocks, coords.front()))
                || (z == CHUNK_DIMENSIONS - 1
                    && (front
                        .map(|c| check(c, block, actual_block, blocks, ChunkBlockCoordinate::new(x, y, 0)))
                        .unwrap_or(true)))
            {
                faces.push(BlockFace::Back);
            }
            // back
            if (z != 0 && check(lod, block, actual_block, blocks, coords.back().expect("Checked in first condition")))
                || (z == 0
                    && (back
                        .map(|c| {
                            check(
                                c,
                                block,
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
                let block = blocks.from_numeric_id(block);

                let Some(material) = materials.get_value(block) else {
                    continue;
                };

                let Some(mesh) = meshes.get_value(block) else {
                    continue;
                };

                if !self.meshes.contains_key(&material.handle) {
                    self.meshes.insert(material.handle.clone(), Default::default());
                }

                let mesh_builder = self.meshes.get_mut(&material.handle).unwrap();

                let rotation = block_info.get_rotation();

                for face in faces.iter().map(|x| BlockFace::rotate_face(*x, rotation)) {
                    let index = block_textures
                        .from_id(block.unlocalized_name())
                        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                    let maybe_img_idx = if self.scale > 8.0 {
                        index
                            .atlas_index("lod")
                            .map(|x| Some(x))
                            .unwrap_or_else(|| index.atlas_index_from_face(face))
                    } else {
                        index.atlas_index_from_face(face)
                    };

                    let Some(image_index) = maybe_img_idx else {
                        warn!("Missing image index -- {index:?}");
                        continue;
                    };

                    let uvs = atlas.uvs_for_index(image_index);

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
                        uvs,
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

            mesh_materials.push(MeshMaterial { material, mesh });
        }

        LodMesh {
            mesh_materials,
            scale: self.scale,
        }
    }
}

#[derive(Component, Debug, Reflect, Default)]
struct LodMeshes(Vec<Entity>);

fn recursively_process_lod(
    lod: &mut Lod,
    offset: Vec3,
    to_process: &Mutex<Option<Vec<(Entity, LodMesh, Vec3)>>>,
    entity: Entity,
    atlas: &MainAtlas,
    blocks: &Registry<Block>,
    materials: &ManyToOneRegistry<Block, CosmosMaterial>,
    meshes_registry: &BlockMeshRegistry,
    block_textures: &Registry<BlockTextureIndex>,
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
                    entity,
                    atlas,
                    blocks,
                    materials,
                    meshes_registry,
                    block_textures,
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
                &atlas,
                &materials,
                &lod_chunk,
                None,
                None,
                None,
                None,
                None,
                None,
                &blocks,
                &meshes_registry,
                &block_textures,
            );

            let mut mutex = to_process.lock().expect("Error locking to_process vec!");

            mutex.as_mut().unwrap().push((
                entity,
                renderer.create_mesh(),
                Vec3::new(
                    offset.x * CHUNK_DIMENSIONSF,
                    offset.y * CHUNK_DIMENSIONSF,
                    offset.z * CHUNK_DIMENSIONSF,
                ),
            ));
        }
    };
}

fn find_non_dirty(lod: &Lod, offset: Vec3, to_process: &mut Vec<Vec3>, scale: f32) {
    match lod {
        Lod::None => {}
        Lod::Children(children) => {
            children.iter().enumerate().for_each(|(i, c)| {
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

                find_non_dirty(c, offset, to_process, scale / 2.0);
            });
        }
        Lod::Single(_, dirty) => {
            if *dirty {
                return;
            }

            to_process.push(offset);
        }
    };
}

/// Performance hot spot
fn monitor_lods_needs_rendered_system(
    mut commands: Commands,
    atlas: Res<MainAtlas>,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<Registry<Block>>,
    materials: Res<ManyToOneRegistry<Block, CosmosMaterial>>,
    meshes_registry: Res<BlockMeshRegistry>,
    chunk_meshes_query: Query<&LodMeshes>,
    block_textures: Res<Registry<BlockTextureIndex>>,
    transform_query: Query<&Transform>,

    mut lods_needed: Query<(Entity, &mut Lod, &Structure), Changed<Lod>>,
) {
    // by making the Vec an Option<Vec> I can take ownership of it later, which I cannot do with
    // just a plain Mutex<Vec>.
    // https://stackoverflow.com/questions/30573188/cannot-move-data-out-of-a-mutex
    let to_process: Mutex<Option<Vec<(Entity, LodMesh, Vec3)>>> = Mutex::new(Some(Vec::new()));
    let to_keep: Mutex<Option<HashMap<Entity, Vec<Vec3>>>> = Mutex::new(Some(HashMap::new()));

    let mut todo = Vec::from_iter(lods_needed.iter_mut());

    // Render lods in parallel
    todo.par_iter_mut().for_each(|(entity, lod, structure)| {
        let scale = structure.chunk_dimensions().x as f32;

        let mut non_dirty = vec![];
        find_non_dirty(lod, Vec3::ZERO, &mut non_dirty, scale);

        to_keep
            .lock()
            .expect("failed to lock mutex")
            .as_mut()
            .unwrap()
            .insert(*entity, non_dirty);

        recursively_process_lod(
            lod.as_mut(),
            Vec3::ZERO,
            &to_process,
            *entity,
            &atlas,
            &blocks,
            &materials,
            &meshes_registry,
            &block_textures,
            scale,
        );
    });

    let to_process_chunks = to_process.lock().unwrap().take().unwrap();

    let mut ent_meshes = HashMap::new();
    for (entity, chunk_mesh, offset) in to_process_chunks {
        if !ent_meshes.contains_key(&entity) {
            ent_meshes.insert(entity, vec![]);
        }
        ent_meshes.get_mut(&entity).expect("Just added").push((chunk_mesh, offset));
    }

    for (entity, mut lod_meshes) in ent_meshes {
        let mut old_mesh_entities = Vec::new();

        let to_keep_locations = to_keep.lock().unwrap().take().unwrap_or_default();

        let to_keep_locations = to_keep_locations.get(&entity);

        if let Ok(chunk_meshes_component) = chunk_meshes_query.get(entity) {
            for ent in chunk_meshes_component.0.iter() {
                let old_mesh_handle = mesh_query.get(*ent).expect("This should have a mesh component.");

                if let Some(old_mesh_handle) = old_mesh_handle {
                    meshes.remove(old_mesh_handle);
                }

                old_mesh_entities.push(*ent);
            }
        }

        let mut entities_to_add = Vec::new();

        // If the structure previously only had one chunk mesh, then it would be on
        // the structure entity instead of child entities
        commands
            .entity(entity)
            .remove::<Handle<Mesh>>()
            .remove::<Handle<StandardMaterial>>();

        let mut structure_meshes_component = LodMeshes::default();

        let single = lod_meshes.len() == 1 && lod_meshes.first().map(|(x, _)| x.mesh_materials.len() == 1).unwrap_or(false);

        if !single {
            for (lod_mesh, offset) in lod_meshes {
                for mesh_material in lod_mesh.mesh_materials {
                    let mesh = meshes.add(mesh_material.mesh);

                    let s = (CHUNK_DIMENSIONS / 2) as f32 * lod_mesh.scale;

                    let ent = if let Some(ent) = old_mesh_entities.pop() {
                        commands.entity(ent).insert((
                            TransformBundle::from_transform(Transform::from_translation(offset)),
                            mesh,
                            mesh_material.material,
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                        ));

                        ent
                    } else {
                        let s = (CHUNK_DIMENSIONS / 2) as f32 * lod_mesh.scale;

                        let ent = commands
                            .spawn((
                                PbrBundle {
                                    mesh,
                                    material: mesh_material.material,
                                    transform: Transform::from_translation(offset),
                                    ..Default::default()
                                },
                                NotShadowCaster,
                                // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                                Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                            ))
                            .id();

                        entities_to_add.push(ent);

                        ent
                    };

                    structure_meshes_component.0.push(ent);
                }
            }
        } else {
            // To avoid making too many entities (and tanking performance), if only one mesh
            // is present, just stick the mesh info onto the chunk itself.

            // offset (_) will always be Vec3::ZERO when there is just one
            let (chunk_mesh, _) = &mut lod_meshes[0];
            let mesh_material = chunk_mesh.mesh_materials.pop().expect("This has one element in it");

            let mesh = meshes.add(mesh_material.mesh);
            let s = (CHUNK_DIMENSIONS / 2) as f32 * chunk_mesh.scale;

            commands.entity(entity).insert((
                mesh,
                mesh_material.material,
                NotShadowCaster,
                // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
            ));
        }

        // Any leftover entities are useless now, so kill them
        for mesh_entity in old_mesh_entities {
            if let Ok(transform) = transform_query.get(mesh_entity) {
                if to_keep_locations.map(|x| x.contains(&transform.translation)).unwrap_or(false) {
                    structure_meshes_component.0.push(mesh_entity);
                    continue;
                }
            }

            commands.entity(mesh_entity).despawn_recursive();
        }

        let mut entity_commands = commands.entity(entity);

        for ent in entities_to_add {
            entity_commands.add_child(ent);
        }

        entity_commands
            // .insert(meshes.add(chunk_mesh.mesh))
            .insert(structure_meshes_component);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (monitor_lods_needs_rendered_system).run_if(in_state(GameState::Playing)));
}
