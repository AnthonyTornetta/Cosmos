use bevy::{platform::collections::HashMap, prelude::*};
use serde::Serialize;
use serde_versioning::Deserialize;

use crate::{netty::cosmos_encoder, physics::location::Location};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
struct BlueprintOld {
    data: HashMap<String, Vec<u8>>,
    /// Used to identify the location this should be saved under
    location: Option<Location>,
    should_save: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Reflect)]
#[versioning(previous_version = BlueprintOld)]
pub struct Blueprint {
    name: String,
    kind: BlueprintType,
    /// Represents the data part of the [`SerializedData`] component on the server.
    serialized_data: Vec<u8>,
}

impl TryFrom<BlueprintOld> for Blueprint {
    type Error = ();

    fn try_from(value: BlueprintOld) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            serialized_data: cosmos_encoder::serialize_uncompressed(&value.data),
            name: "Blueprint".into(),
            kind: BlueprintType::Ship,
        })
    }
}

impl Blueprint {
    pub fn new(serialized_data: Vec<u8>, name: String, blueprint_type: BlueprintType) -> Self {
        Self {
            serialized_data,
            name,
            kind: blueprint_type,
        }
    }

    pub fn serialized_data(&self) -> &[u8] {
        self.serialized_data.as_slice()
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum BlueprintType {
    Ship,
    Station,
    Asteroid,
}

impl BlueprintType {
    pub fn blueprint_directory(&self) -> &'static str {
        match self {
            Self::Ship => "ship",
            Self::Station => "station",
            Self::Asteroid => "asteroid",
        }
    }

    pub fn path_for(&self, blueprint_name: &str) -> String {
        format!("blueprints/{}/{}.bp", self.blueprint_directory(), blueprint_name)
    }
}

pub(super) fn register(app: &mut App) {}
