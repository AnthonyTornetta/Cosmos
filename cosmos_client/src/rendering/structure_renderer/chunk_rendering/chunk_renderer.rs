use crate::asset::asset_loading::{BlockNeighbors, BlockTextureIndex};
use crate::asset::materials::{BlockMaterialMapping, MaterialDefinition};
use crate::block::lighting::{BlockLightProperties, BlockLighting};
use crate::rendering::structure_renderer::{BlockRenderingModes, RenderingMode};
use bevy::ecs::event::Event;
use bevy::log::warn;
use bevy::prelude::{App, Deref, DerefMut, Entity, Rect, Resource, Vec3};
use bevy::tasks::Task;
use bevy::utils::hashbrown::HashMap;
use cosmos_core::block::{block_direction::BlockDirection, Block};
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::many_to_one::ManyToOneRegistry;
use cosmos_core::registry::Registry;
use cosmos_core::structure::block_storage::BlockStorer;
use cosmos_core::structure::chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use cosmos_core::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate};
use cosmos_core::utils::array_utils::expand;
use std::collections::HashSet;

use super::{BlockMeshRegistry, ChunkMesh, ChunkRenderResult, MeshBuilder, MeshInfo, MeshMaterial};

#[derive(Default, Debug)]
pub struct ChunkRenderer {
    meshes: HashMap<u16, MeshInfo>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    pub fn render(
        &mut self,
        materials: &ManyToOneRegistry<Block, BlockMaterialMapping>,
        materials_registry: &Registry<MaterialDefinition>,
        lighting: &Registry<BlockLighting>,
        chunk: &Chunk,
        left: Option<&Chunk>,
        right: Option<&Chunk>,
        bottom: Option<&Chunk>,
        top: Option<&Chunk>,
        back: Option<&Chunk>,
        front: Option<&Chunk>,
        blocks: &Registry<Block>,
        meshes: &BlockMeshRegistry,
        rendering_modes: &BlockRenderingModes,
        block_textures: &Registry<BlockTextureIndex>,
    ) -> HashSet<u16> {
        let cd2 = CHUNK_DIMENSIONSF / 2.0;

        let mut faces = Vec::with_capacity(6);

        let mut custom_blocks = HashSet::new();

        for (coords, (block_id, block_info)) in chunk
            .blocks()
            .copied()
            .zip(chunk.block_info_iterator().copied())
            .enumerate()
            .map(|(i, block)| {
                (
                    ChunkBlockCoordinate::from(expand(i, CHUNK_DIMENSIONS as usize, CHUNK_DIMENSIONS as usize)),
                    block,
                )
            })
            .filter(|(coords, _)| chunk.has_block_at(*coords))
        {
            let rendering_mode = rendering_modes.rendering_mode(block_id);

            if rendering_mode == RenderingMode::Both || rendering_mode == RenderingMode::Custom {
                custom_blocks.insert(block_id);
            }

            if rendering_mode == RenderingMode::Custom {
                // If this is custom rendered, we shouldn't do the normal rendering logic here.
                continue;
            }

            let (center_offset_x, center_offset_y, center_offset_z) = (
                coords.x as f32 - cd2 + 0.5,
                coords.y as f32 - cd2 + 0.5,
                coords.z as f32 - cd2 + 0.5,
            );
            let actual_block = blocks.from_numeric_id(block_id);

            let check_should_render = |c: &Chunk,
                                       actual_block: &Block,
                                       blocks: &Registry<Block>,
                                       coords: ChunkBlockCoordinate,
                                       should_connect: &mut bool|
             -> bool {
                let block_id_here = c.block_at(coords);
                let block_here = blocks.from_numeric_id(block_id_here);
                *should_connect = actual_block.should_connect_with(block_here);

                let custom_rendered = rendering_modes.rendering_mode(block_id_here);

                // A block adjacent is custom
                custom_rendered == RenderingMode::Custom
                    || (!(actual_block.is_fluid() && block_here == actual_block)
                        && (block_here.is_see_through() || !actual_block.is_full()))
            };

            let (x, y, z) = (coords.x, coords.y, coords.z);

            let mut block_connections = [false; 6];

            // Positive X.
            if (x != CHUNK_DIMENSIONS - 1
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.pos_x(),
                    &mut block_connections[BlockDirection::PosX.index()],
                ))
                || (x == CHUNK_DIMENSIONS - 1
                    && (right
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(0, y, z),
                                &mut block_connections[BlockDirection::PosX.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockDirection::PosX);
            }
            // Negative X.
            if (x != 0
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.neg_x().expect("Checked in first condition"),
                    &mut block_connections[BlockDirection::NegX.index()],
                ))
                || (x == 0
                    && (left
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(CHUNK_DIMENSIONS - 1, y, z),
                                &mut block_connections[BlockDirection::NegX.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockDirection::NegX);
            }

            // Positive Y.
            if (y != CHUNK_DIMENSIONS - 1
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.pos_y(),
                    &mut block_connections[BlockDirection::PosY.index()],
                ))
                || (y == CHUNK_DIMENSIONS - 1
                    && top
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, 0, z),
                                &mut block_connections[BlockDirection::PosY.index()],
                            )
                        })
                        .unwrap_or(true))
            {
                faces.push(BlockDirection::PosY);
            }
            // Negative Y.
            if (y != 0
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.neg_y().expect("Checked in first condition"),
                    &mut block_connections[BlockDirection::NegY.index()],
                ))
                || (y == 0
                    && (bottom
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, CHUNK_DIMENSIONS - 1, z),
                                &mut block_connections[BlockDirection::NegY.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockDirection::NegY);
            }

            // Positive Z.
            if (z != CHUNK_DIMENSIONS - 1
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.pos_z(),
                    &mut block_connections[BlockDirection::PosZ.index()],
                ))
                || (z == CHUNK_DIMENSIONS - 1
                    && (front
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, y, 0),
                                &mut block_connections[BlockDirection::PosZ.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockDirection::PosZ);
            }
            // Negative Z.
            if (z != 0
                && check_should_render(
                    chunk,
                    actual_block,
                    blocks,
                    coords.neg_z().expect("Checked in first condition"),
                    &mut block_connections[BlockDirection::NegZ.index()],
                ))
                || (z == 0
                    && (back
                        .map(|c| {
                            check_should_render(
                                c,
                                actual_block,
                                blocks,
                                ChunkBlockCoordinate::new(x, y, CHUNK_DIMENSIONS - 1),
                                &mut block_connections[BlockDirection::NegZ.index()],
                            )
                        })
                        .unwrap_or(true)))
            {
                faces.push(BlockDirection::NegZ);
            }

            if !faces.is_empty() {
                let block = blocks.from_numeric_id(block_id);

                let Some(material) = materials.get_value(block) else {
                    continue;
                };

                let mat_id = material.material_id();

                let Some(mesh) = meshes.get_value(block) else {
                    continue;
                };

                if !self.meshes.contains_key(&mat_id) {
                    self.meshes.insert(mat_id, Default::default());
                }

                let material_definition = materials_registry.from_numeric_id(mat_id);

                let mesh_builder = self.meshes.get_mut(&mat_id).unwrap();

                let block_rotation = block_info.get_rotation();

                let rotation = block_rotation.as_quat();

                for (direction, face) in faces
                    .iter()
                    .map(|direction| (*direction, block_rotation.block_face_pointing(*direction)))
                {
                    // println!(
                    //     "{:?}: Block face {:?} rendered pointing {:?} due to rotation {:?}",
                    //     coords, face, direction, block_rotation
                    // );

                    let mut one_mesh_only = false;

                    let Some(mut mesh_info) = mesh
                        .info_for_face(face, block_connections[direction.index()])
                        .map(Some)
                        .unwrap_or_else(|| {
                            let single_mesh = mesh.info_for_whole_block();

                            if single_mesh.is_some() {
                                one_mesh_only = true;
                            }

                            single_mesh
                        })
                        .cloned()
                    else {
                        // This face has no model, ignore
                        continue;
                    };

                    let index = block_textures
                        .from_id(block.unlocalized_name())
                        .unwrap_or_else(|| block_textures.from_id("missing").expect("Missing texture should exist."));

                    let mut neighbors = BlockNeighbors::empty();

                    match direction {
                        BlockDirection::PosZ | BlockDirection::NegZ => {
                            if block_connections[BlockDirection::PosX.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockDirection::NegX.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockDirection::PosY.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockDirection::NegY.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                        BlockDirection::PosY | BlockDirection::NegY => {
                            if block_connections[BlockDirection::PosX.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockDirection::NegX.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockDirection::PosZ.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockDirection::NegZ.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                        // idk why right and left have to separate, and I don't want to know why
                        BlockDirection::PosX => {
                            if block_connections[BlockDirection::PosZ.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockDirection::NegZ.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockDirection::PosY.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockDirection::NegY.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                        BlockDirection::NegX => {
                            if block_connections[BlockDirection::NegZ.index()] {
                                neighbors |= BlockNeighbors::Right;
                            }
                            if block_connections[BlockDirection::PosZ.index()] {
                                neighbors |= BlockNeighbors::Left;
                            }
                            if block_connections[BlockDirection::PosY.index()] {
                                neighbors |= BlockNeighbors::Top;
                            }
                            if block_connections[BlockDirection::NegY.index()] {
                                neighbors |= BlockNeighbors::Bottom;
                            }
                        }
                    }

                    let Some(image_index) = index.atlas_index_from_face(face, neighbors) else {
                        warn!("Missing image index for face {direction} -- {index:?}");
                        continue;
                    };

                    let uvs = Rect::new(0.0, 0.0, 1.0, 1.0);

                    for pos in mesh_info.positions.iter_mut() {
                        *pos = rotation.mul_vec3((*pos).into()).into();
                    }

                    for norm in mesh_info.normals.iter_mut() {
                        *norm = rotation.mul_vec3((*norm).into()).into();
                    }

                    let additional_info = material_definition.add_material_data(block_id, &mesh_info);

                    mesh_builder.add_mesh_information(
                        &mesh_info,
                        Vec3::new(center_offset_x, center_offset_y, center_offset_z),
                        uvs,
                        image_index,
                        additional_info,
                    );

                    if one_mesh_only {
                        break;
                    }
                }

                faces.clear();

                if let Some(lighting) = lighting.from_id(block.unlocalized_name()) {
                    self.lights.insert(coords, lighting.properties);
                }
            }
        }

        custom_blocks
    }

    pub fn create_mesh(self) -> ChunkMesh {
        let mut mesh_materials = Vec::new();

        for (material, chunk_mesh_info) in self.meshes {
            let mesh = chunk_mesh_info.build_mesh();

            mesh_materials.push(MeshMaterial {
                material_id: material,
                mesh,
            });
        }

        let lights = self.lights;

        ChunkMesh { lights, mesh_materials }
    }
}

#[derive(Debug)]
pub(super) struct RenderingChunk(pub Task<ChunkRenderResult>);

#[derive(Resource, Debug, DerefMut, Deref, Default)]
pub(super) struct RenderingChunks(pub Vec<RenderingChunk>);

#[derive(Event, Eq)]
pub struct ChunkNeedsCustomBlocksRendered {
    pub structure_entity: Entity,
    pub chunk_coordinate: ChunkCoordinate,
    pub mesh_entity_parent: Entity,
    pub block_ids: HashSet<u16>,
}

impl PartialEq for ChunkNeedsCustomBlocksRendered {
    fn eq(&self, other: &Self) -> bool {
        self.structure_entity == other.structure_entity && self.chunk_coordinate == other.chunk_coordinate
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ChunkNeedsCustomBlocksRendered>().init_resource::<RenderingChunks>();
}
