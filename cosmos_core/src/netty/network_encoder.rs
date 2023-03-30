use serde::{Deserialize, Serialize};

/// Serializes the data to be sent - compresses it if needed
pub fn serialize<T: Serialize>(x: &T) -> Vec<u8> {
    let data = bincode::serialize(x).expect("Error serializing data!");

    if data.len() > 50 {
        zstd::encode_all(data.as_slice(), 0).expect("Error compressing data!")
    } else {
        data
    }
}

/// Deserializes the data - will decompress if needed
///
/// Will change raw to be the uncompressed form if it is compressed.
pub fn deserialize<'a, T: Deserialize<'a>>(raw: &'a mut Vec<u8>) -> T {
    if raw.len() > 50 {
        *raw = zstd::decode_all(raw.as_slice()).expect("Error decompressing data!");
    }

    bincode::deserialize::<T>(raw).expect("Error deserializing data!")
}
