//! Handles both the saving & loading of entities on the server

use std::{
    fmt::Display,
    fs,
    sync::{Arc, Mutex},
};

use bevy::{
    prelude::{App, Component, Resource},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};
use rand::{distributions::Alphanumeric, Rng};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmos_core::{
    physics::location::{Location, Sector},
    structure::chunk::netty::SaveData,
};

pub mod loading;
pub mod player_loading;
pub mod saving;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
/// NOT ALL ENTITIES WILL HAVE THIS ON THEM!
///
/// Only entities that have been loaded or saved will have this. This is a unique identifier for
/// this entity.
pub struct EntityId(String);

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Resource, Default, Clone)]
/// This is a resource that caches the saved entities of different sectors that a player has been near.
///
/// This is just used to prevent excessive IO operations.
pub struct SectorsCache(Arc<Mutex<HashMap<Sector, Arc<Mutex<HashSet<(EntityId, Option<u32>)>>>>>>);

impl SectorsCache {
    /// Gets all the entities that are saved for this sector in this cache.  This does NOT
    /// perform any IO operations.
    pub fn get(&self, sector: &Sector) -> Option<Arc<Mutex<HashSet<(EntityId, Option<u32>)>>>> {
        self.0.lock().expect("Failed to lock").get(sector).cloned()
    }

    /// Removes a saved entity from this location - this does not do any IO operations,
    /// and only removes it from the cache.
    pub fn remove(&mut self, entity_id: &EntityId, sector: Sector, load_distance: Option<u32>) {
        if let Some(set) = self.0.lock().expect("Failed to lock").get_mut(&sector) {
            set.lock().expect("Failed to unlock").remove(&(entity_id.clone(), load_distance));
        }
    }

    /// Inserts an entity into this sector in this cache. This does not perform any IO operations.
    pub fn insert(&mut self, sector: Sector, entity_id: EntityId, load_distance: Option<u32>) {
        let self_locked = &mut self.0.lock().expect("Failed to lock");

        if !self_locked.contains_key(&sector) {
            self_locked.insert(sector, Arc::new(Mutex::new(HashSet::new())));
        }

        self_locked
            .get_mut(&sector)
            .expect("Sector doesn't exist despite me just making it")
            .lock()
            .expect("Failed to unlock")
            .insert((entity_id, load_distance));
    }
}

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

#[derive(Debug, Clone)]
pub(crate) enum SaveFileIdentifierType {
    /// This entity does not belong to any other entity
    Base(EntityId, Option<Sector>, Option<u32>),
    SubEntity(Box<SaveFileIdentifier>, EntityId),
    /// Denotes that this entity belongs to another entity, and should be saved
    /// in that entity's folder. Once this entity is saved, the [`SaveFileIdentifierType`] component will be removed.
    ///
    /// ## Note:
    /// While saving is handled for you, it is up to you to load this yourself.
    ///
    /// This will be saved to `world/x_y_z/belongsToEntityId/thisEntityId.cent`
    BelongsTo(Box<SaveFileIdentifier>, String),
}

#[derive(Debug, Component, Clone)]
/// Used to track where the save file for a given entity is or should be.
pub struct SaveFileIdentifier {
    identifier_type: SaveFileIdentifierType,
}

impl SaveFileIdentifier {
    /// Creates a new SaveFileIdentifier from this location & entity id
    pub fn new(sector: Option<Sector>, entity_id: EntityId, load_distance: Option<u32>) -> Self {
        Self {
            identifier_type: SaveFileIdentifierType::Base(entity_id, sector, load_distance),
        }
    }

    /// Creates a new SaveFileIdentifier from this location & entity id
    pub fn sub_entity(parent_save_file_identifier: SaveFileIdentifier, entity_id: EntityId) -> Self {
        Self {
            identifier_type: SaveFileIdentifierType::SubEntity(Box::new(parent_save_file_identifier), entity_id),
        }
    }

    /// If this SaveFileIdentifier is a base identifier (not child),
    /// this will return its EntityId. Otherwise, returns None.
    pub fn entity_id(&self) -> Option<&EntityId> {
        match &self.identifier_type {
            SaveFileIdentifierType::Base(entity_id, _, _) => Some(entity_id),
            SaveFileIdentifierType::SubEntity(_, entity_id) => Some(entity_id),
            _ => None,
        }
    }

    /// Creates a new SaveFileIdentifier from this location & entity id
    pub fn as_child(this_identifier: impl Into<String>, belongs_to: SaveFileIdentifier) -> Self {
        Self {
            identifier_type: SaveFileIdentifierType::BelongsTo(Box::new(belongs_to), this_identifier.into()),
        }
    }

    /// Gets the file path a given entity will be saved to.
    pub fn get_save_file_path(&self) -> String {
        format!("{}.cent", self.get_save_file_directory(Self::get_save_file_name))
    }

    /// Gets the save file name without the .cent extension, but not the whole path
    fn get_save_file_name(&self) -> String {
        match &self.identifier_type {
            SaveFileIdentifierType::Base(entity, _, load_distance) => load_distance
                .map(|ld| format!("{ld}_{}", entity.as_str()))
                .unwrap_or(entity.as_str().to_owned()),
            SaveFileIdentifierType::SubEntity(_, entity_id) => entity_id.as_str().to_owned(),
            SaveFileIdentifierType::BelongsTo(_, name) => name.to_owned(),
        }
    }

    /// Gets the save file name without the .cent extension, but not the whole path
    fn get_save_file_name_no_load_distance(&self) -> String {
        match &self.identifier_type {
            SaveFileIdentifierType::Base(entity, _, _) => entity.as_str().to_owned(),
            SaveFileIdentifierType::SubEntity(_, entity_id) => entity_id.as_str().to_owned(),
            SaveFileIdentifierType::BelongsTo(_, name) => name.to_owned(),
        }
    }

    /// Gets the save file name, but not the whole path
    fn get_save_file_directory(&self, base_get_save_file_name: impl Fn(&Self) -> String) -> String {
        match &self.identifier_type {
            SaveFileIdentifierType::Base(_, sector, _) => {
                let directory = sector.map(Self::get_sector_path).unwrap_or("world/nowhere".into());

                format!("{directory}/{}", base_get_save_file_name(self))
            }
            SaveFileIdentifierType::SubEntity(belongs_to, _) => {
                format!(
                    "{}/{}",
                    belongs_to.get_save_file_directory(Self::get_save_file_name_no_load_distance),
                    base_get_save_file_name(self)
                )
            }
            SaveFileIdentifierType::BelongsTo(belongs_to, _) => {
                format!(
                    "{}/{}",
                    belongs_to.get_save_file_directory(Self::get_save_file_name_no_load_distance),
                    base_get_save_file_name(self)
                )
            }
        }
    }

    /// Gets the directory path all children of this entity would be saved to
    pub fn get_children_directory(&self) -> String {
        self.get_save_file_directory(Self::get_save_file_name_no_load_distance)
    }

    /// Gets the directory for this sector's save folder
    fn get_sector_path(sector: Sector) -> String {
        let (x, y, z) = (sector.x(), sector.y(), sector.z());

        format!("world/{x}_{y}_{z}")
    }
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize)]
/// Stores the serialized data for an entity.
///
/// This is either read from or written to a save file depending on if an entity is being loaded or saved.
pub struct SerializedData {
    save_data: SaveData,

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
            save_data: SaveData::default(),
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
            self.save_data.save(data_id, data);
        }
    }

    /// Calls `cosmos_encoder::serialize` on the passed in data.
    /// Then sends that data into the `save` method, with the given data id.
    ///
    /// Will only serialize & save if `should_save()` returns true.

    pub fn serialize_data(&mut self, data_id: impl Into<String>, data: &impl Serialize) {
        if self.should_save() {
            self.save_data.serialize_data(data_id, data);
        }
    }

    /// Reads the data as raw bytes at the given data id. Use `deserialize_data` for a streamlined way to read the data.
    pub fn read_data(&self, data_id: &str) -> Option<&Vec<u8>> {
        self.save_data.read_data(data_id)
    }

    /// Deserializes the data as the given type (via `cosmos_encoder::deserialize`) at the given id. Will panic if the
    /// data is not properly serialized.
    pub fn deserialize_data<T: DeserializeOwned>(&self, data_id: &str) -> Option<T> {
        self.save_data.deserialize_data(data_id)
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
pub fn is_sector_generated(sector: Sector) -> bool {
    fs::try_exists(SaveFileIdentifier::get_sector_path(sector)).unwrap_or(false)
}

pub(super) fn register(app: &mut App) {
    saving::register(app);
    loading::register(app);
    player_loading::register(app);

    app.register_type::<EntityId>().register_type::<SerializedData>();
}
