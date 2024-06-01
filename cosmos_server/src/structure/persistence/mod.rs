//! Saving/reading from disk block data

use bevy::{
    app::App,
    ecs::{component::Component, system::Commands},
    log::warn,
};
use cosmos_core::structure::{
    chunk::netty::{SerializedBlockData, SerializedChunkBlockData},
    structure_iterator::ChunkIteratorResult,
    Structure,
};

use crate::persistence::{saving::NeedsSaved, SerializedData};

pub mod chunk;

#[derive(Component, Debug, Clone, Copy)]
/// Signifies that this block's data needs saved
pub(crate) struct BlockDataNeedsSaved;

pub(crate) fn save_structure(structure: &Structure, s_data: &mut SerializedData, commands: &mut Commands) {
    s_data.serialize_data("cosmos:structure", structure);

    for chunk in structure.all_chunks_iter(false) {
        let ChunkIteratorResult::FilledChunk { position, chunk: _ } = chunk else {
            unreachable!();
        };

        let Some(chunk) = structure.chunk_at(position) else {
            warn!("Missing chunk but tried to save it!");
            continue;
        };

        let mut has_block_data_to_save = false;
        for (_, &entity) in chunk.all_block_data_entities() {
            commands.entity(entity).insert(BlockDataNeedsSaved);
            has_block_data_to_save = true;
        }

        if has_block_data_to_save {
            if let Some(chunk_ent) = structure.chunk_entity(position) {
                commands.entity(chunk_ent).insert((SerializedBlockData::new(position), NeedsSaved));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    chunk::register(app);
}
