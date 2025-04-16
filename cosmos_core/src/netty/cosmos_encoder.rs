//! Use this instead of bincode to serialize & deserialize things.
//!
//! This compresses items before their usage & decompresses them before deserializing to save a ton
//! of space + bits sent over the network.

use bevy::log::error;
use bincode::config::{Fixint, LittleEndian, NoLimit};
use serde::{Serialize, de::DeserializeOwned};

const CONFIG: bincode::config::Configuration<LittleEndian, Fixint, NoLimit> = bincode::config::legacy();

/// Serializes the data to be sent - compresses it if needed
pub fn serialize<T: Serialize>(x: &T) -> Vec<u8> {
    let data = serialize_uncompressed(x);

    lz4_flex::compress_prepend_size(data.as_slice())
}

/// Serializes data without apply any form of compression
pub fn serialize_uncompressed<T: Serialize>(x: &T) -> Vec<u8> {
    bincode::serde::encode_to_vec(x, CONFIG).expect("Error serializing data")
}

/// Deserializes data assuming it has not been compressed
pub fn deserialize_uncompressed<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Box<bincode::error::DecodeError>> {
    let (res, _) = bincode::serde::decode_from_slice(bytes, CONFIG)?;

    Ok(res)
}

/// Deserializes the data - will decompress if needed
pub fn deserialize<T: DeserializeOwned>(raw: &[u8]) -> Result<T, Box<bincode::error::DecodeError>> {
    let Ok(decompressed) = lz4_flex::decompress_size_prepended(raw) else {
        return Err(Box::new(bincode::error::DecodeError::Other("Unable to decompress".into())));
    };

    let res = deserialize_uncompressed(&decompressed);

    if res.is_err() {
        error!("Error deserializing - decompressed form: {:?}", decompressed);
    }

    res
}
