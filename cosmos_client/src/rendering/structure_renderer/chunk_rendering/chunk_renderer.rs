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
use cosmos_core::structure::chunk::{CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use cosmos_core::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate};
use cosmos_core::utils::array_utils::expand;
use std::collections::HashSet;

use super::neighbor_checking::ChunkRendererBackend;
use super::{BlockMeshRegistry, ChunkMesh, ChunkRenderResult, MeshBuilder, MeshInfo, MeshMaterial};

#[derive(Default, Debug)]
pub struct ChunkRenderer<M: MeshBuilder + Default> {
    meshes: HashMap<(u16, u32), MeshInfo<M>>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

impl<M: MeshBuilder + Default> ChunkRenderer<M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders a chunk into mesh information that can then be turned into a bevy mesh
    pub fn render<C: BlockStorer, R: ChunkRendererBackend<C>>(
        &mut self,
        materials_registry: &ManyToOneRegistry<Block, BlockMaterialMapping>,
        materials_definition_registry: &Registry<MaterialDefinition>,
        lighting: &Registry<BlockLighting>,
        chunk: &C,
        blocks: &Registry<Block>,
        meshes: &BlockMeshRegistry,
        rendering_modes: &BlockRenderingModes,
        block_textures: &Registry<BlockTextureIndex>,
        rendering_backend: &R,
        scale: f32,
        offset: Vec3,
        lod: bool,
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

                if rendering_mode == RenderingMode::Custom {
                    // If this is custom rendered, we shouldn't do the normal rendering logic here.
                    continue;
                }
            }

            let (center_offset_x, center_offset_y, center_offset_z) = (
                (coords.x as f32 - cd2 + 0.5) * scale,
                (coords.y as f32 - cd2 + 0.5) * scale,
                (coords.z as f32 - cd2 + 0.5) * scale,
            );
            let block_here = blocks.from_numeric_id(block_id);

            let mut block_connections = [false; 6];

            let mut check_rendering = |direction: BlockDirection| {
                if rendering_backend.check_should_render(
                    chunk,
                    block_here,
                    coords,
                    blocks,
                    direction,
                    &mut block_connections[direction.index()],
                    rendering_modes,
                ) {
                    faces.push(direction);
                }
            };

            check_rendering(BlockDirection::PosX);
            check_rendering(BlockDirection::PosY);
            check_rendering(BlockDirection::PosZ);
            check_rendering(BlockDirection::NegX);
            check_rendering(BlockDirection::NegY);
            check_rendering(BlockDirection::NegZ);

            if !faces.is_empty() {
                let block = blocks.from_numeric_id(block_id);

                let material_definition = if !lod {
                    let Some(material) = materials_registry.get_value(block) else {
                        continue;
                    };

                    let mat_id = material.material_id();

                    materials_definition_registry.from_numeric_id(mat_id)
                } else {
                    materials_definition_registry.from_id("cosmos:lod").expect("Missing LOD material.")
                };

                let Some(mesh) = meshes.get_value(block) else {
                    continue;
                };

                let block_rotation = block_info.get_rotation();

                let rotation = block_rotation.as_quat();

                let mut mesh_builder = None;

                for (direction, face) in faces
                    .iter()
                    .map(|direction| (*direction, block_rotation.block_face_pointing(*direction)))
                {
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

                    let Some(image_index) = rendering_backend.get_texture_index(index, neighbors, face) else {
                        warn!("Missing image index for face {direction} -- {index:?}");
                        continue;
                    };

                    let uvs = Rect::new(0.0, 0.0, 1.0, 1.0);

                    for pos in mesh_info.positions.iter_mut() {
                        let position_vec3 =
                            rendering_backend.transform_position(chunk, coords, direction, rotation.mul_vec3(Vec3::from(*pos) * scale));
                        *pos = (offset + position_vec3).into();
                    }

                    for norm in mesh_info.normals.iter_mut() {
                        *norm = rotation.mul_vec3((*norm).into()).into();
                    }

                    let additional_info = material_definition.add_material_data(block_id, &mesh_info);

                    if mesh_builder.is_none() {
                        mesh_builder = Some(
                            self.meshes
                                .entry((material_definition.id(), image_index.dimension_index))
                                .or_default(),
                        );
                    }

                    mesh_builder.as_mut().unwrap().add_mesh_information(
                        &mesh_info,
                        Vec3::new(center_offset_x, center_offset_y, center_offset_z),
                        uvs,
                        image_index.texture_index,
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

        for ((material, texture_dimensions_index), chunk_mesh_info) in self.meshes {
            let mesh = chunk_mesh_info.build_mesh();

            mesh_materials.push(MeshMaterial {
                material_id: material,
                texture_dimensions_index,
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
