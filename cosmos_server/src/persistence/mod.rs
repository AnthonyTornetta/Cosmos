use bevy::{
    prelude::{App, Component, Resource},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmos_core::{netty::cosmos_encoder, physics::location::Location};

pub mod loading;
pub mod player_loading;
pub mod saving;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Hash)]
pub struct EntityId(String);

#[derive(Debug, Resource, Default)]
pub struct SectorsCache(HashMap<(i64, i64, i64), HashSet<EntityId>>);

impl EntityId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

#[derive(Debug, Component, Reflect)]
pub struct SaveFileIdentifier {
    pub sector: Option<(i64, i64, i64)>,
    pub entity_id: EntityId,
}

impl SaveFileIdentifier {
    /// Gets the file path a given entity will be saved to.
    pub fn get_save_file_path(&self) -> String {
        let directory = self
            .sector
            .map(|(x, y, z)| format!("{x}_{y}_{z}"))
            .unwrap_or("nowhere".into());

        format!("world/{directory}/{}.cent", self.entity_id.0)
    }

    pub fn get_save_file_name(&self) -> String {
        format!("{}.cent", self.entity_id.0)
    }
}

#[derive(Component, Debug, Reflect, Serialize, Deserialize)]
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

pub(super) fn register(app: &mut App) {
    saving::register(app);
    loading::register(app);
    player_loading::register(app);

    app.register_type::<EntityId>()
        .register_type::<SaveFileIdentifier>();
}
