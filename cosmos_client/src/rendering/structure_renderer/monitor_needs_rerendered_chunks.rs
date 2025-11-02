use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use cosmos_core::events::block_events::{BlockChangedMessage, BlockDataChangedMessage};
use cosmos_core::structure::Structure;
use cosmos_core::structure::chunk::CHUNK_DIMENSIONS;
use cosmos_core::structure::coordinates::ChunkCoordinate;
use cosmos_core::structure::events::ChunkSetMessage;
use std::collections::HashSet;

use super::chunk_rendering::ChunkNeedsRendered;
use super::{BlockDataRerenderOnChange, StructureRenderingSet};

fn monitor_block_updates_system(
    mut evr_block_changed: MessageReader<BlockChangedMessage>,
    mut evr_chunk_set_event: MessageReader<ChunkSetMessage>,
    mut evr_changed_data: MessageReader<BlockDataChangedMessage>,
    q_structure: Query<&Structure>,
    q_block_data_rerender_flag: Query<(), With<BlockDataRerenderOnChange>>,
    mut commands: Commands,
) {
    let mut chunks_todo = HashMap::<Entity, HashSet<ChunkCoordinate>>::default();

    for ev in evr_changed_data.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };

        if structure.query_block_data(ev.block.coords(), &q_block_data_rerender_flag).is_none() {
            continue;
        }

        let chunks = chunks_todo.entry(ev.block.structure()).or_default();

        let cc = ev.block.chunk_coords();

        if ev.block.x() != 0 && ev.block.x().is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }

        let dims = structure.block_dimensions();

        if ev.block.x() != dims.x - 1 && (ev.block.x() + 1).is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }

        if ev.block.y() != 0 && ev.block.y().is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }

        if ev.block.y() != dims.y - 1 && (ev.block.y() + 1).is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }

        if ev.block.z() != 0 && ev.block.z().is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }

        if ev.block.z() != dims.z - 1 && (ev.block.z() + 1).is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z + 1));
        }

        chunks.insert(cc);
    }

    for ev in evr_block_changed.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };

        let chunks = chunks_todo.entry(ev.block.structure()).or_default();

        let cc = ev.block.chunk_coords();

        if ev.block.x() != 0 && ev.block.x().is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x - 1, cc.y, cc.z));
        }

        let dims = structure.block_dimensions();

        if ev.block.x() != dims.x - 1 && (ev.block.x() + 1).is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x + 1, cc.y, cc.z));
        }

        if ev.block.y() != 0 && ev.block.y().is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y - 1, cc.z));
        }

        if ev.block.y() != dims.y - 1 && (ev.block.y() + 1).is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y + 1, cc.z));
        }

        if ev.block.z() != 0 && ev.block.z().is_multiple_of(CHUNK_DIMENSIONS) {
            chunks.insert(ChunkCoordinate::new(cc.x, cc.y, cc.z - 1));
        }

        if ev.block.z() != dims.z - 1 && (ev.block.z() + 1).is_multiple_of(CHUNK_DIMENSIONS) {
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

            if let Ok(mut chunk_ent) = commands.get_entity(chunk_entity) {
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
