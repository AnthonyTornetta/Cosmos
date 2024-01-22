use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum RegistrySyncing {
    RegistryCount(u64),
    Registry { serialized: Vec<u8>, registry_name: String },
}
