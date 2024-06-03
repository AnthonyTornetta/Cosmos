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

use super::chunk_rendering::ChunkNeedsRendered;
use super::{BlockMeshRegistry, CosmosMeshBuilder, MeshBuilder, MeshInformation, ReadOnlyBlockMeshRegistry, StructureRenderingSet};

fn monitor_block_updates_system(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut evr_chunk_set_event: EventReader<ChunkSetEvent>,
    mut evr_changed_data: EventReader<BlockDataChangedEvent>,
    q_structure: Query<&Structure>,
    mut commands: Commands,
) {
    let mut chunks_todo = HashMap::<Entity, HashSet<ChunkCoordinate>>::default();

    for ev in evr_changed_data.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.structure_entity).or_default();

        let cc = ev.block.chunk_coords();

        if ev.block.x() != 0 && ev.block.x() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }

        let dims = structure.block_dimensions();

        if ev.block.x() != dims.x - 1 && (ev.block.x() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }

        if ev.block.y() != 0 && ev.block.y() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }

        if ev.block.y() != dims.y - 1 && (ev.block.y() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }

        if ev.block.z() != 0 && ev.block.z() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }

        if ev.block.z() != dims.z - 1 && (ev.block.z() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }

        chunks.insert(cc);
    }

    for ev in evr_block_changed.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.structure_entity).or_default();

        let cc = ev.block.chunk_coords();

        if ev.block.x() != 0 && ev.block.x() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }

        let dims = structure.block_dimensions();

        if ev.block.x() != dims.x - 1 && (ev.block.x() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }

        if ev.block.y() != 0 && ev.block.y() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }

        if ev.block.y() != dims.y - 1 && (ev.block.y() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }

        if ev.block.z() != 0 && ev.block.z() % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }

        if ev.block.z() != dims.z - 1 && (ev.block.z() + 1) % CHUNK_DIMENSIONS == 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }

        chunks.insert(cc);
    }

    for ev in evr_chunk_set_event.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.structure_entity).or_default();

        let cc = ev.coords;

        chunks.insert(cc);

        let dims = structure.chunk_dimensions();

        if cc.z != 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }
        if cc.z < dims.z - 1 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }
        if cc.y != 0 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }
        if cc.y < dims.y - 1 {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }
        if cc.x != 0 {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }
        if cc.x < dims.x - 1 {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }
    }

    for (structure, chunks) in chunks_todo {
        let Ok(structure) = q_structure.get(structure) else {
            continue;
        };

        for coords in chunks {
            let Some(chunk_entity) = structure.chunk_entity(coords) else {
                continue;
            };

            if let Some(mut chunk_ent) = commands.get_entity(chunk_entity) {
                chunk_ent.insert(ChunkNeedsRendered);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        monitor_block_updates_system.in_set(StructureRenderingSet::MonitorBlockUpdates),
    );
}
