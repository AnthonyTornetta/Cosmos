//! Contains shared blueprint logic + data

use bevy::prelude::*;
use renet::ClientId;
use serde::{Deserialize, Serialize};
// use serde_versioning::;

use crate::{faction::FactionId, physics::location::Location, structure::persistence::SaveData};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
/// The old format blueprints were serialized with. DO NOT USE THIS.
pub struct BlueprintOld {
    data: SaveData,
    location: Option<Location>,
    should_save: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Reflect)]
// #[versioning(previous_version = BlueprintOld, pessimistic)]
/// Contains the data about how a structure should be created, such as the raw structure data.
///
/// Also contains metadata about the blueprint
pub struct Blueprint {
    name: String,
    kind: BlueprintType,
    serialized_data: SaveData,
    author: BlueprintAuthor,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Reflect, Default)]
/// Who created a blueprint
pub enum BlueprintAuthor {
    /// A player created this blueprint
    Player {
        /// That player's name (could be out of date)
        name: String,
        /// That player's steam id
        id: ClientId,
    },
    #[default]
    /// Created by a server administrator
    Server,
    /// Created by an NPC faction
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
    /// Creates a new blueprint from this data
    pub fn new(serialized_data: SaveData, name: String, blueprint_type: BlueprintType, author: BlueprintAuthor) -> Self {
        Self {
            serialized_data,
            name,
            kind: blueprint_type,
            author,
        }
    }

    /// Returns the type of this blueprint
    pub fn kind(&self) -> BlueprintType {
        self.kind
    }

    /// Returns the serialized blueprint data
    pub fn serialized_data(&self) -> &SaveData {
        &self.serialized_data
    }

    /// Returns the name of this blueprint
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The creator of this blueprint
    pub fn author(&self) -> &BlueprintAuthor {
        &self.author
    }

    /// Sets the author of this blueprint
    pub fn set_author(&mut self, author: BlueprintAuthor) {
        self.author = author;
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
/// The type of blueprint
pub enum BlueprintType {
    /// This is a ship (and should be in the ship area)
    Ship,
    /// This is a station (and should be in the station area)
    Station,
    /// This is a asteroid (and should be in the asteroid area)
    Asteroid,
}

impl BlueprintType {
    /// Returns the blueprint directory this type of blueprint should be saved in
    pub fn blueprint_directory(&self) -> &'static str {
        match self {
            Self::Ship => "ship",
            Self::Station => "station",
            Self::Asteroid => "asteroid",
        }
    }

    /// Returns the full path for this blueprint type
    pub fn path_for(&self, blueprint_name: &str) -> String {
        format!("blueprints/{}/{}.bp", self.blueprint_directory(), blueprint_name)
    }
}
