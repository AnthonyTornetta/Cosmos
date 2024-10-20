//! Use this instead of bincode to serialize & deserialize things.
//!
//! This compresses items before their usage & decompresses them before deserializing to save a ton
//! of space + bits sent over the network.

use bevy::log::error;
use serde::{de::DeserializeOwned, Serialize};

/// Serializes the data to be sent - compresses it if needed
pub fn serialize<T: Serialize>(x: &T) -> Vec<u8> {
    let data = bincode::serialize(x).expect("Error serializing data!");

    lz4_flex::compress_prepend_size(data.as_slice())
}

/// Deserializes the data - will decompress if needed
pub fn deserialize<T: DeserializeOwned>(raw: &[u8]) -> Result<T, Box<bincode::ErrorKind>> {
    let Ok(decompressed) = lz4_flex::decompress_size_prepended(raw) else {
        return Err(Box::new(bincode::ErrorKind::Custom("Unable to decompress".into())));
    };

    let res = bincode::deserialize::<T>(&decompressed);

    if res.is_err() {
        error!("Error deserializing - decompressed form: {:?}", decompressed);
    }

    res
}
