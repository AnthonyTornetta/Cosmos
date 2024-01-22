use serde::{de::DeserializeOwned, Serialize};

use super::StructureSystemImpl;

pub trait SyncableSystem: Serialize + DeserializeOwned + StructureSystemImpl {}
