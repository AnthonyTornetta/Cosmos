use crate::asset::asset_loading::{BlockNeighbors, BlockTextureIndex};
use crate::asset::materials::{
    add_materials, remove_materials, AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType, RemoveAllMaterialsEvent,
};
use crate::block::lighting::{BlockLightProperties, BlockLighting};
use crate::state::game_state::GameState;
use crate::structure::planet::unload_chunks_far_from_players;
use bevy::ecs::event::Event;
use bevy::ecs::schedule::{IntoSystemSetConfigs, OnExit, SystemSet};
use bevy::log::warn;
use bevy::prelude::{
    in_state, App, Assets, BuildChildren, Commands, Component, Deref, DerefMut, DespawnRecursiveExt, Entity, EventReader, EventWriter,
    GlobalTransform, Handle, IntoSystemConfigs, Mesh, PointLight, PointLightBundle, Query, Rect, Res, ResMut, Resource, Transform, Update,
    Vec3, VisibilityBundle, With,
};
use bevy::reflect::Reflect;
use bevy::render::mesh::{MeshVertexAttribute, VertexAttributeValues};
use bevy::render::primitives::Aabb;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::transform::TransformBundle;
use bevy::utils::hashbrown::HashMap;
use cosmos_core::block::{Block, BlockFace};
use cosmos_core::events::block_events::{BlockChangedEvent, BlockDataChangedEvent};
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::physics::location::SECTOR_DIMENSIONS;
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::registry::many_to_one::{ManyToOneRegistry, ReadOnlyManyToOneRegistry};
use cosmos_core::registry::{ReadOnlyRegistry, Registry};
use cosmos_core::structure::block_storage::BlockStorer;
use cosmos_core::structure::chunk::{Chunk, ChunkEntity, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use cosmos_core::structure::coordinates::{ChunkBlockCoordinate, ChunkCoordinate, UnboundChunkCoordinate};
use cosmos_core::structure::events::ChunkSetEvent;
use cosmos_core::structure::Structure;
use cosmos_core::utils::array_utils::expand;
use futures_lite::future;
use std::collections::HashSet;
use std::mem::swap;

use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation, ReadOnlyBlockMeshRegistry};

mod async_rendering;
pub mod chunk_renderer;

#[derive(Debug)]
struct MeshMaterial {
    mesh: Mesh,
    material_id: u16,
}

#[derive(Debug)]
struct ChunkMesh {
    mesh_materials: Vec<MeshMaterial>,
    lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
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

pub(super) fn register(app: &mut App) {
    app.register_type::<LightsHolder>();

    async_rendering::register(app);
}
