use bevy::prelude::*;
use renet::ClientId;
use serde::{Deserialize, Serialize};
// use serde_versioning::;

use crate::{faction::FactionId, physics::location::Location, structure::persistence::SaveData};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
pub struct BlueprintOld {
    data: SaveData,
    location: Option<Location>,
    should_save: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Reflect)]
// #[versioning(previous_version = BlueprintOld, pessimistic)]
pub struct Blueprint {
    name: String,
    kind: BlueprintType,
    serialized_data: SaveData,
    author: BlueprintAuthor,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Reflect, Default)]
pub enum BlueprintAuthor {
    Player {
        name: String,
        id: ClientId,
    },
    #[default]
    Server,
    Faction(FactionId),
}

impl TryFrom<BlueprintOld> for Blueprint {
    type Error = ();

    fn try_from(value: BlueprintOld) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            serialized_data: value.data,
            name: "Blueprint".into(),
            kind: BlueprintType::Ship,
            author: Default::default(),
        })
    }
}

impl Blueprint {
    pub fn new(serialized_data: SaveData, name: String, blueprint_type: BlueprintType, author: BlueprintAuthor) -> Self {
        Self {
            serialized_data,
            name,
            kind: blueprint_type,
            author,
        }
    }

    pub fn kind(&self) -> BlueprintType {
        self.kind
    }

    pub fn serialized_data(&self) -> &SaveData {
        &self.serialized_data
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn author(&self) -> &BlueprintAuthor {
        &self.author
    }

    pub fn set_author(&mut self, author: BlueprintAuthor) {
        self.author = author;
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
