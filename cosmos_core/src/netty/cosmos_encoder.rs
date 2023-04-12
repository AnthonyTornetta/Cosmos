use serde::{de::DeserializeOwned, Serialize};

/// Serializes the data to be sent - compresses it if needed
pub fn serialize<T: Serialize>(x: &T) -> Vec<u8> {
    let data = bincode::serialize(x).expect("Error serializing data!");

    zstd::encode_all(data.as_slice(), 0).expect("Error compressing data!")
}

/// Deserializes the data - will decompress if needed
pub fn deserialize<T: DeserializeOwned>(raw: &[u8]) -> Result<T, Box<bincode::ErrorKind>> {
    let Ok(decompressed) = zstd::decode_all(raw) else {
        return Err(Box::new(bincode::ErrorKind::Custom("Unable to decompress".into())));
    };

    bincode::deserialize::<T>(&decompressed)
}
