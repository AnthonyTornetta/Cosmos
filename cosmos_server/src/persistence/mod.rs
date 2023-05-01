//! Handles both the saving & loading of entities on the server

use std::fs;

use bevy::{
    prelude::{App, Component, Resource},
    reflect::{FromReflect, Reflect},
    utils::{HashMap, HashSet},
};
use rand::{distributions::Alphanumeric, Rng};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmos_core::{netty::cosmos_encoder, physics::location::Location};

pub mod loading;
pub mod player_loading;
pub mod saving;

#[derive(
    Component, Debug, Reflect, FromReflect, Serialize, Deserialize, PartialEq, Eq, Clone, Hash,
)]
/// NOT ALL ENTITIES WILL HAVE THIS ON THEM!
///
/// Only entities that have been loaded or saved will have this. This is a unique identifier for
/// this entity.
pub struct EntityId(String);

#[derive(Debug, Resource, Default)]
/// This is a resource that caches the saved entities of different sectors that a player has been near.
///
/// This is just used to prevent excessive IO operations.
pub struct SectorsCache(HashMap<(i64, i64, i64), HashSet<EntityId>>);

impl EntityId {
    /// Creates a new EntityID.
    ///
    /// * `id` This should be unique to only this entity. If this isn't unique, the entity may not be loaded/saved correctly
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Creates a new EntityId
    pub fn generate() -> Self {
        Self::new(
            rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect::<String>(),
        )
    }

    /// Returns the entity id as a string
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

// /// Denotes that this entity belongs to another entity, and should be saved
// /// in that entity's folder. Once this entity is saved, this component will be removed.
// ///
// /// ## Note:
// /// While saving is handled for you, it is up to you to load this yourself.
// ///
// /// This will be saved to `world/x_y_z/belongsToEntityId/thisEntityId.cent`
// #[derive(Component, Debug, Reflect, FromReflect, Clone)]
// pub struct BelongsTo {
//     /// The entity id this belongs to
//     pub entity_id: EntityId,

//     location: Location,
// }

// impl BelongsTo {
//     /// Creates a belongs to relationship with this entity id
//     pub fn new(entity_id: EntityId, location: Location) -> Self {
//         Self {
//             entity_id,
//             location,
//         }
//     }
// }

#[derive(Debug, Clone)]
pub(crate) enum SaveFileIdentifierType {
    Base((EntityId, Option<(i64, i64, i64)>)),
    BelongsTo((Box<SaveFileIdentifier>, String)),
}

#[derive(Debug, Component, Clone)]
/// Used to track where the save file for a given entity is or should be.
pub struct SaveFileIdentifier {
    identifier_type: SaveFileIdentifierType,
}

impl SaveFileIdentifier {
    /// Creates a new SaveFileIdentifier from this location & entity id
    pub fn new(sector: Option<(i64, i64, i64)>, entity_id: EntityId) -> Self {
        Self {
            identifier_type: SaveFileIdentifierType::Base((entity_id, sector)),
        }
    }

    /// Creates a new SaveFileIdentifier from this location & entity id
    pub fn as_child(this_identifier: String, belongs_to: SaveFileIdentifier) -> Self {
        Self {
            identifier_type: SaveFileIdentifierType::BelongsTo((
                Box::new(belongs_to),
                this_identifier,
            )),
        }
    }

    /// Gets the file path a given entity will be saved to.
    ///
    /// `world/X_Y_Z/entity_id.cent`
    pub fn get_save_file_path(&self) -> String {
        format!("{}.cent", self.get_save_file_directory(),)
    }

    /// Gets the save file name without the .cent extension, but not the whole path
    ///
    /// `entity_id`
    pub fn get_save_file_name(&self) -> String {
        match &self.identifier_type {
            SaveFileIdentifierType::Base((entity, _)) => entity.as_str().to_owned(),
            SaveFileIdentifierType::BelongsTo((_, name)) => name.to_owned(),
        }
    }

    /// Gets the save file name, but not the whole path
    ///
    /// `entity_id.cent`
    pub fn get_save_file_directory(&self) -> String {
        match &self.identifier_type {
            SaveFileIdentifierType::Base((_, sector)) => {
                let directory = sector
                    .map(|sector| Self::get_sector_path(sector))
                    .unwrap_or("world/nowhere".into());

                format!("{directory}/{}", self.get_save_file_name())
            }
            SaveFileIdentifierType::BelongsTo((belongs_to, _)) => {
                format!(
                    "{}/{}",
                    belongs_to.get_save_file_directory(),
                    self.get_save_file_name()
                )
            }
        }
    }

    /// Gets the directory for this sector's save folder
    pub fn get_sector_path(sector: (i64, i64, i64)) -> String {
        let (x, y, z) = sector;

        format!("world/{x}_{y}_{z}")
    }
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize)]
/// Stores the serialized data for an entity.
///
/// This is either read from or written to a save file depending on if an entity is being loaded or saved.
pub struct SerializedData {
    save_data: HashMap<String, Vec<u8>>,

    /// Used to identify the location this should be saved under
    location: Option<Location>,
    should_save: bool,
}

impl SerializedData {
    /// Use this to set location. This will make sure the folder name
    /// reflects the actual location.
    pub fn set_location(&mut self, loc: &Location) {
        self.serialize_data("cosmos:location", loc);
        self.location = Some(*loc);
    }
}

impl Default for SerializedData {
    fn default() -> Self {
        Self {
            save_data: HashMap::default(),
            location: None,
            should_save: true,
        }
    }
}

impl SerializedData {
    /// Saves the data to that data id. Will overwrite any existing data at that id.
    ///
    /// Will only save if `should_save()` returns true.
    pub fn save(&mut self, data_id: impl Into<String>, data: Vec<u8>) {
        if self.should_save() {
            self.save_data.insert(data_id.into(), data);
        }
    }

    /// Calls `cosmos_encoder::serialize` on the passed in data.
    /// Then sends that data into the `save` method, with the given data id.
    ///
    /// Will only serialize & save if `should_save()` returns true.

    pub fn serialize_data(&mut self, data_id: impl Into<String>, data: &impl Serialize) {
        if self.should_save() {
            self.save(data_id, cosmos_encoder::serialize(data));
        }
    }

    /// Reads the data as raw bytes at the given data id. Use `deserialize_data` for a streamlined way to read the data.
    pub fn read_data(&self, data_id: &str) -> Option<&Vec<u8>> {
        self.save_data.get(data_id)
    }

    /// Deserializes the data as the given type (via `cosmos_encoder::deserialize`) at the given id. Will panic if the
    /// data is not properly serialized.
    pub fn deserialize_data<T: DeserializeOwned>(&self, data_id: &str) -> Option<T> {
        self.read_data(data_id)
            .map(|d| cosmos_encoder::deserialize(d).expect("Error deserializing data!"))
    }

    /// Sets whether this should actually be saved - if false, when save and serialize_data is called,
    /// nothing will happen.
    pub fn set_should_save(&mut self, should_save: bool) {
        self.should_save = should_save;
    }

    /// If this is false, no data will be saved/serialized when `save` and `serialize_data` is called.
    ///
    /// No data will be written to the disk either if this is false.
    pub fn should_save(&self) -> bool {
        self.should_save
    }
}

/// Returns true if a sector has at some point been generated at this location
pub fn is_sector_loaded(sector: (i64, i64, i64)) -> bool {
    fs::try_exists(SaveFileIdentifier::get_sector_path(sector)).unwrap_or(false)
}

pub(super) fn register(app: &mut App) {
    saving::register(app);
    loading::register(app);
    player_loading::register(app);

    app.register_type::<EntityId>();
}
