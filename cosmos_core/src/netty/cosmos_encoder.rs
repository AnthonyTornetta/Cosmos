use serde::{de::DeserializeOwned, Serialize};

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
pub fn deserialize<'a, T: DeserializeOwned>(raw: &[u8]) -> Result<T, Box<bincode::ErrorKind>> {
    if raw.len() > 50 {
        let Ok(decompressed) = zstd::decode_all(raw) else {
            return Err(Box::new(bincode::ErrorKind::Custom("Unable to decompress".into())));
        };

        bincode::deserialize::<T>(&decompressed)
    } else {
        bincode::deserialize::<T>(raw)
    }
}
