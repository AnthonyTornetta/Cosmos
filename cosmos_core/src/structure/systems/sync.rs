//! Syncs systems from server -> client

use serde::{de::DeserializeOwned, Serialize};

use super::StructureSystemImpl;

/// Implemenet this trait to make it serializable
pub trait SyncableSystem: Serialize + DeserializeOwned + StructureSystemImpl {}
