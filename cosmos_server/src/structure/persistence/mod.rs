//! Saving/reading from disk block data

use bevy::{
    app::App,
    ecs::{component::Component, system::Commands},
    log::{info, warn},
    prelude::{Deref, DerefMut},
    utils::HashMap,
};
use cosmos_core::structure::{
    coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
    structure_iterator::ChunkIteratorResult,
    Structure,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::persistence::{saving::NeedsSaved, SaveData, SerializedData};

pub mod chunk;

#[derive(DerefMut, Deref, Debug, Serialize, Deserialize, Default)]
pub struct SerializedChunkBlockData(HashMap<ChunkBlockCoordinate, SaveData>);

#[derive(Component, Debug, Serialize, Deserialize)]
/// This will *eventually* be a component on the chunk that stores all the block data that has been serialized.
pub struct SerializedBlockData {
    /// The chunk's coordinates stored for your convenience
    pub chunk: ChunkCoordinate,
    /// If this is being saved for a blueprint instead of an actual world file
    save_data: SerializedChunkBlockData,
}

impl SerializedBlockData {
    /// Creates an empty serialized data field
    pub fn new(chunk: ChunkCoordinate) -> Self {
        Self {
            chunk,
            save_data: Default::default(),
        }
    }

    /// Saves the data to that data id. Will overwrite any existing data at that id.
    ///
    /// Will only save if `should_save()` returns true.
    pub fn save(&mut self, block: ChunkBlockCoordinate, data_id: impl Into<String>, data: Vec<u8>) {
        self.save_data.entry(block).or_default().save(data_id.into(), data);
    }

    /// Calls `cosmos_encoder::serialize` on the passed in data.
    /// Then sends that data into the `save` method, with the given data id.
    ///
    /// Will only serialize & save if `should_save()` returns true.

    pub fn serialize_data(&mut self, block: ChunkBlockCoordinate, data_id: impl Into<String>, data: &impl Serialize) {
        self.save_data.entry(block).or_default().serialize_data(data_id, data);
    }

    /// Reads the data as raw bytes at the given data id. Use `deserialize_data` for a streamlined way to read the data.
    pub fn read_data(&self, block: ChunkBlockCoordinate, data_id: &str) -> Option<&Vec<u8>> {
        if let Some(save_data) = self.save_data.get(&block) {
            save_data.read_data(data_id)
        } else {
            None
        }
    }

    /// Deserializes the data as the given type (via `cosmos_encoder::deserialize`) at the given id. Will panic if the
    /// data is not properly serialized.
    pub fn deserialize_data<T: DeserializeOwned>(&self, block: ChunkBlockCoordinate, data_id: &str) -> Option<T> {
        if let Some(save_data) = self.save_data.get(&block) {
            save_data.deserialize_data(data_id)
        } else {
            None
        }
    }

    pub(crate) fn take_save_data(&mut self) -> SerializedChunkBlockData {
        std::mem::take(&mut self.save_data)
    }
}

#[derive(Component, Debug, Clone, Copy)]
/// Signifies that this block's data needs saved
pub(crate) struct BlockDataNeedsSaved;

pub(crate) fn save_structure(structure: &Structure, s_data: &mut SerializedData, commands: &mut Commands) {
    s_data.serialize_data("cosmos:structure", structure);

    for chunk in structure.all_chunks_iter(false) {
        let ChunkIteratorResult::FilledChunk { position, chunk: _ } = chunk else {
            unreachable!();
        };

        let Some(chunk) = structure.chunk_from_chunk_coordinates(position) else {
            warn!("Missing chunk but tried to save it!");
            continue;
        };

        let mut has_block_data_to_save = false;
        for (_, &entity) in chunk.all_block_data_entities() {
            info!("Logging block's components!");

            commands.entity(entity).insert(BlockDataNeedsSaved);
            has_block_data_to_save = true;
        }

        if has_block_data_to_save {
            if let Some(chunk_ent) = structure.chunk_entity(position) {
                info!("SAID SAVE THIS CHUNK'S BLOCK DATA!");

                commands.entity(chunk_ent).insert((SerializedBlockData::new(position), NeedsSaved));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    chunk::register(app);
}
