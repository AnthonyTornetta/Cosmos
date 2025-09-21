//! Shared persistence logic between client + server.

use bevy::{platform::collections::HashMap, prelude::*};
use derive_more::derive::{Display, Error};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::netty::cosmos_encoder;

#[derive(Debug, Reflect, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
/// Stores a mapping if `id` -> `serialized, uncompressed data`.
///
/// To extract your component, use
/// [`save_data.deserialize_data::<Component>("modid:component_unlocalized_name")`]
pub struct SaveData(pub HashMap<String, Vec<u8>>);

impl SaveData {
    /// Saves the data to that data id. Will overwrite any existing data at that id.
    ///
    /// Will only save if `should_save()` returns true.
    pub fn save(&mut self, data_id: impl Into<String>, data: Vec<u8>) {
        self.0.insert(data_id.into(), data);
    }

    /// Calls `cosmos_encoder::serialize` on the passed in data.
    /// Then sends that data into the `save` method, with the given data id.
    ///
    /// Will only serialize & save if `should_save()` returns true.
    pub fn serialize_data(&mut self, data_id: impl Into<String>, data: &impl Serialize) {
        self.save(data_id, cosmos_encoder::serialize(data));
    }

    /// Reads the data as raw bytes at the given data id. Use `deserialize_data` for a streamlined way to read the data.
    pub fn read_data(&self, data_id: &str) -> Option<&Vec<u8>> {
        self.0.get(data_id)
    }

    /// Deserializes the data as the given type (via `cosmos_encoder::deserialize`) at the given id. If there is no id of this type,
    /// will return [`DeserializationError::NoEntry`]. Will return [`DeserializationError::ErrorParsing`] if the
    /// data is not properly serialized.
    pub fn deserialize_data<T: DeserializeOwned>(&self, data_id: &str) -> Result<T, DeserializationError> {
        let Some(data) = self.read_data(data_id) else {
            return Err(DeserializationError::NoEntry);
        };

        cosmos_encoder::deserialize(data).map_err(DeserializationError::ErrorParsing)
    }
}

#[derive(Error, Display, Debug)]
/// Unable to deserialize the given data
pub enum DeserializationError {
    /// Something went wrong deserializing the binary. This meant something bad happened.
    ErrorParsing(Box<bincode::error::DecodeError>),
    /// There is no entry for this serialized id.
    NoEntry,
}
