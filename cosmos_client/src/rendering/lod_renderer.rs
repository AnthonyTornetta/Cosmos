use std::{f32::consts::PI, sync::Mutex};

use bevy::{prelude::*, render::primitives::Aabb, utils::HashMap};
use cosmos_core::{
    block::{Block, BlockFace},
    registry::{identifiable::Identifiable, many_to_one::ManyToOneRegistry, Registry},
    structure::{
        chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF},
        coordinates::ChunkBlockCoordinate,
        lod::Lod,
        lod_chunk::LodChunk,
        Structure,
    },
    utils::array_utils::expand,
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    asset::asset_loading::{BlockTextureIndex, MainAtlas},
    materials::CosmosMaterial,
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation};

#[derive(Debug)]
struct MeshMaterial {
    mesh: Mesh,
    material: Handle<StandardMaterial>,
}

#[derive(Debug)]
struct ChunkMesh {
    mesh_materials: Vec<MeshMaterial>,
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
}

impl ChunkRenderer {
    fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    fn render(
        &mut self,
        scale: f32,
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

                    let Some(image_index) = index.atlas_index_from_face(face) else {
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

                    mesh_builder.add_mesh_information(&mesh_info, Vec3::new(center_offset_x, center_offset_y, center_offset_z), uvs);

                    if one_mesh_only {
                        break;
                    }
                }

                faces.clear();
            }
        }
    }

    fn create_mesh(self) -> ChunkMesh {
        let mut mesh_materials = Vec::new();

        for (material, chunk_mesh_info) in self.meshes {
            let mesh = chunk_mesh_info.build_mesh();

            mesh_materials.push(MeshMaterial { material, mesh });
        }

        ChunkMesh { mesh_materials }
    }
}

#[derive(Component, Debug, Reflect, Default)]
struct ChunkMeshes(Vec<Entity>);

/// Performance hot spot
fn monitor_lods_needs_rendered_system(
    mut commands: Commands,
    atlas: Res<MainAtlas>,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    blocks: Res<Registry<Block>>,
    materials: Res<ManyToOneRegistry<Block, CosmosMaterial>>,
    meshes_registry: Res<BlockMeshRegistry>,
    chunk_meshes_query: Query<&ChunkMeshes>,
    block_textures: Res<Registry<BlockTextureIndex>>,

    chunks_need_rendered: Query<(Entity, &Lod), Changed<Lod>>,
) {
    // by making the Vec an Option<Vec> I can take ownership of it later, which I cannot do with
    // just a plain Mutex<Vec>.
    // https://stackoverflow.com/questions/30573188/cannot-move-data-out-of-a-mutex
    let to_process: Mutex<Option<Vec<(Entity, ChunkMesh)>>> = Mutex::new(Some(Vec::new()));

    let todo = Vec::from_iter(chunks_need_rendered.iter());

    // Render lods in parallel
    todo.par_iter().for_each(|(entity, lod)| {
        let mut renderer = ChunkRenderer::new();

        match lod {
            Lod::None => {}
            Lod::Children(_) => panic!("Not done yet"),
            Lod::Single(lod_chunk) => {
                renderer.render(
                    1.0,
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

                mutex.as_mut().unwrap().push((*entity, renderer.create_mesh()));
            }
        };
    });

    let to_process_chunks = to_process.lock().unwrap().take().unwrap();

    for (entity, mut chunk_mesh) in to_process_chunks {
        let mut old_mesh_entities = Vec::new();

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

        // meshes

        // If the structure previously only had one chunk mesh, then it would be on
        // the structure entity instead of child entities
        commands
            .entity(entity)
            .remove::<Handle<Mesh>>()
            .remove::<Handle<StandardMaterial>>();

        let mut structure_meshes_component = ChunkMeshes::default();

        if chunk_mesh.mesh_materials.len() > 1 {
            for mesh_material in chunk_mesh.mesh_materials {
                let mesh = meshes.add(mesh_material.mesh);

                let ent = if let Some(ent) = old_mesh_entities.pop() {
                    commands.entity(ent).insert(mesh).insert(mesh_material.material);

                    ent
                } else {
                    let s = (CHUNK_DIMENSIONS / 2) as f32;

                    let ent = commands
                        .spawn((
                            PbrBundle {
                                mesh,
                                material: mesh_material.material,
                                ..Default::default()
                            },
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                        ))
                        .id();

                    entities_to_add.push(ent);

                    ent
                };

                structure_meshes_component.0.push(ent);
            }
        } else if !chunk_mesh.mesh_materials.is_empty() {
            // To avoid making too many entities (and tanking performance), if only one mesh
            // is present, just stick the mesh info onto the chunk itself.

            let mesh_material = chunk_mesh.mesh_materials.pop().expect("This has one element in it");

            let mesh = meshes.add(mesh_material.mesh);
            let s = (CHUNK_DIMENSIONS / 2) as f32;

            commands.entity(entity).insert((
                mesh,
                mesh_material.material,
                // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (bevy 0.12 released)
                Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
            ));
        }

        // Any leftover entities are useless now, so kill them
        for mesh in old_mesh_entities {
            commands.entity(mesh).despawn_recursive();
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
