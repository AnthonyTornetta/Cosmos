//! Contains shared blueprint logic + data

use bevy::prelude::*;
use renet::ClientId;
use serde::{Deserialize, Serialize};
// use serde_versioning::;

use crate::{
    faction::FactionId,
    physics::location::Location,
    structure::{coordinates::BlockCoordinate, persistence::SaveData},
};

/// SaveData key containing docked child structures for composite blueprints.
pub const COMPOSITE_BLUEPRINT_DATA_KEY: &str = "cosmos:blueprint_docked_children";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
/// Extra blueprint payload containing structures docked to the root structure.
pub struct CompositeBlueprint {
    /// Recursively docked child structures. The root structure is always index 0.
    pub children: Vec<CompositeBlueprintChild>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
/// A docked child structure in a composite blueprint.
pub struct CompositeBlueprintChild {
    /// Blueprint-local index for this structure.
    pub index: u32,
    /// Blueprint-local index of the structure this child is docked to.
    pub parent_index: u32,
    /// The child structure kind.
    pub blueprint_type: BlueprintType,
    /// The child structure's normal serialized structure data.
    pub serialized_data: SaveData,
    /// Dock information for reconnecting this child to its parent.
    pub docked: CompositeBlueprintDocked,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
/// Serialized dock information for a composite blueprint child.
pub struct CompositeBlueprintDocked {
    /// The block on the parent entity this child is docked to.
    pub to_block: BlockCoordinate,
    /// The block on this child entity that acts as the docking block.
    pub this_block: BlockCoordinate,

    /// Relative to the parent entity.
    pub relative_rotation: Quat,
    /// Relative translation to the parent entity.
    pub relative_translation: Vec3,

    /// If this docked structure can rotate about this axis relative to the parent.
    pub rotate_x: bool,
    /// If this docked structure can rotate about this axis relative to the parent.
    pub rotate_y: bool,
    /// If this docked structure can rotate about this axis relative to the parent.
    pub rotate_z: bool,

    /// Where, relative to the parent, this child is docked/anchored to.
    pub parent_anchor: Vec3,
    /// Where, relative to the child, this child is docked/anchored to.
    pub child_anchor: Vec3,
}

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
