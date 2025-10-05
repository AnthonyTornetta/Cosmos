//! Responbible for the storage a chunks' block data, used for both network communications & saving/reading chunks from disk.

use bevy::{platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::structure::{
    coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
    persistence::*,
};

#[derive(DerefMut, Deref, Debug, Serialize, Deserialize, Default)]
/// Contains all the serialized block data for each block that has it in a chunk
pub struct SerializedChunkBlockData(HashMap<ChunkBlockCoordinate, SaveData>);

#[derive(Component, Debug, Serialize, Deserialize)]
/// This is a component on the chunk that stores all the block data that has been serialized.
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
    pub fn deserialize_data<T: DeserializeOwned>(&self, block: ChunkBlockCoordinate, data_id: &str) -> Result<T, DeserializationError> {
        if let Some(save_data) = self.save_data.get(&block) {
            save_data.deserialize_data(data_id)
        } else {
            Err(DeserializationError::NoEntry)
        }
    }

    /// Takes the current save data & replaces it w/ the default.
    ///
    /// Only used to efficiently use the data after everything has been serialized.
    pub fn take_save_data(&mut self) -> SerializedChunkBlockData {
        std::mem::take(&mut self.save_data)
    }
}
