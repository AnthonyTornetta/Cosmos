//! Use this instead of bincode to serialize & deserialize things.
//!
//! This compresses items before their usage & decompresses them before deserializing to save a ton
//! of space + bits sent over the network.

use bevy::log::error;
use bincode::{Decode, Encode};

const CONFIG: bincode::config::Configuration = bincode::config::standard();

/// Serializes the data to be sent - compresses it if needed
pub fn serialize<T: Encode>(x: &T) -> Vec<u8> {
    let data = serialize_uncompressed(x);

    lz4_flex::compress_prepend_size(data.as_slice())
}

pub fn serialize_uncompressed<T: Encode>(x: &T) -> Vec<u8> {
    bincode::encode_to_vec(x, CONFIG).expect("Error serializing data")
}

pub fn deserialize_uncompressed<T: Decode<()>>(bytes: &[u8]) -> Result<T, Box<bincode::error::DecodeError>> {
    let (res, _) = bincode::decode_from_slice(bytes, CONFIG)?;

    Ok(res)
}

/// Deserializes the data - will decompress if needed
pub fn deserialize<T: Decode<()>>(raw: &[u8]) -> Result<T, Box<bincode::error::DecodeError>> {
    let Ok(decompressed) = lz4_flex::decompress_size_prepended(raw) else {
        return Err(Box::new(bincode::error::DecodeError::Other("Unable to decompress".into())));
    };

    let res = deserialize_uncompressed(&decompressed);

    if res.is_err() {
        error!("Error deserializing - decompressed form: {:?}", decompressed);
    }

    res
}
