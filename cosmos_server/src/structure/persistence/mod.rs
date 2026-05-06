//! Saving/reading from disk block data

use bevy::{
    app::App,
    ecs::{component::Component, system::Commands},
    log::{error, warn},
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::{
        Structure,
        chunk::netty::{SerializedBlockData, SerializedChunkBlockData},
        persistence::{SaveData, palette::Palette},
        structure_iterator::ChunkIteratorResult,
    },
};

use crate::persistence::{SerializedData, saving::NeedsSaved};

pub mod chunk;

#[derive(Component, Debug, Clone, Copy)]
/// Signifies that this block's data needs saved
pub(crate) struct BlockDataNeedsSaved;

pub(crate) fn save_structure(structure: &Structure, s_data: &mut SerializedData, blocks: &Registry<Block>, commands: &mut Commands) {
    let palette = Palette::new_from_structure(structure, blocks);

    s_data.serialize(structure);
    s_data.serialize(&palette);

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

        if has_block_data_to_save && let Some(chunk_ent) = structure.chunk_entity(position) {
            commands.entity(chunk_ent).insert((SerializedBlockData::new(position), NeedsSaved));
        }
    }
}

/// Loads this structure while using the [`Palette`] if present in the serialized data. If no
/// palette is present, the structure is assumed to already have the correct blocks.
pub fn load_structure_with_palette(s_data: &SaveData, blocks: &Registry<Block>) -> Option<Structure> {
    let mut structure = s_data.deserialize_identifiable::<Structure>().ok()?;
    if let Ok(palette) = s_data.deserialize_identifiable::<Palette>() {
        let to_change = structure.all_blocks_iter(false).collect::<Vec<_>>();

        for coord in to_change {
            let id = structure.block_id_at(coord);
            let Some(name) = palette.get(id) else {
                error!("Missing palette mapping for {id} @ {coord}! Not loading structure.");
                return None;
            };

            let Some(block) = blocks.from_id(name) else {
                error!("Invalid block id - {name} @ {coord}! Not loading structure.");
                return None;
            };

            let info = structure.block_info_at(coord);

            structure.set_block_and_info_at(coord, block, info, blocks, None);
        }
    }
    Some(structure)
}

pub(super) fn register(app: &mut App) {
    chunk::register(app);
}
