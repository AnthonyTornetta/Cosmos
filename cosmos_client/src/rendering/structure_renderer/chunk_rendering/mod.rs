use crate::block::lighting::BlockLightProperties;
use bevy::platform::collections::HashMap;
use bevy::prelude::{App, Component, Entity, Mesh, Rect, Vec3};
use bevy::reflect::Reflect;
use bevy::render::mesh::{MeshVertexAttribute, VertexAttributeValues};
use cosmos_core::structure::coordinates::ChunkBlockCoordinate;
use std::collections::HashSet;

use super::{BlockMeshRegistry, MeshBuilder, MeshInformation};

mod async_rendering;
pub mod chunk_renderer;
pub mod lod_rendering;
pub mod neighbor_checking;

#[derive(Debug)]
pub struct MeshMaterial {
    pub mesh: Mesh,
    pub material_id: u16,
    pub texture_dimensions_index: u32,
}

#[derive(Debug)]
pub struct ChunkMesh {
    pub mesh_materials: Vec<MeshMaterial>,
    pub lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
}

#[derive(Debug, Reflect, Clone, Copy)]
struct LightEntry {
    entity: Entity,
    light: BlockLightProperties,
    position: ChunkBlockCoordinate,
    valid: bool,
}

#[derive(Component, Debug, Reflect, Default)]
struct LightsHolder {
    lights: Vec<LightEntry>,
}

#[derive(Component, Debug, Reflect, Default)]
/// Keeps track of all children of a Chunk that are holding its meshes + materials
struct ChunkMeshes(Vec<Entity>);

#[derive(Debug)]
struct ChunkRenderResult {
    chunk_entity: Entity,
    /// Any blocks that need their own rendering logic applied to them
    custom_blocks: HashSet<u16>,
    mesh: ChunkMesh,
}

#[derive(Component)]
pub(super) struct ChunkNeedsRendered;

#[derive(Default, Debug)]
struct MeshInfo<M: MeshBuilder> {
    mesh_builder: M,
}

impl<M: MeshBuilder> MeshBuilder for MeshInfo<M> {
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

pub(super) fn register(app: &mut App) {
    app.register_type::<LightsHolder>();

    async_rendering::register(app);
    chunk_renderer::register(app);
}
