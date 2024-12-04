//! Used to sync registries from client -> server

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
/// Used to inform the server that the client has received all necessary registries
///
/// For this to work, both the client and server must call their own versions of `sync_registry<T>` for the registry type.
pub enum RegistrySyncing {
    /// The client has received all necessary registries
    FinishedReceivingRegistries,
}
