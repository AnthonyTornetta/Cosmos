//! Used to sync registries from server -> client

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Used to sync registries from server -> client
///
/// For this to work, both the client and server must call their own versions of `sync_registry<T>` for the registry type.
pub enum RegistrySyncing {
    /// The # of registries the client must received before starting the game
    RegistryCount(u64),
    /// A registry the client must use before starting the game
    Registry {
        /// The serialized form of this registry (serialized via `cosmos_encoder::serialize`)
        serialized: Vec<u8>,
        /// The unlocalized name of this registry
        registry_name: String,
    },
}
