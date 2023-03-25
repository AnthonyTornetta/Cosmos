use bevy::{
    prelude::{App, Component, Resource},
    reflect::Reflect,
    utils::{HashMap, HashSet},
};
use serde::{Deserialize, Serialize};

use cosmos_core::physics::location::Location;

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
}

#[derive(Component, Debug, Default, Reflect, Serialize, Deserialize)]
pub struct SerializedData {
    save_data: HashMap<String, Vec<u8>>,

    /// Used to identify the location this should be saved under
    location: Option<Location>,
}

impl SerializedData {
    /// Saves the data to that data id. Will overwrite any existing data at that id.
    pub fn save(&mut self, data_id: impl Into<String>, data: Vec<u8>) {
        self.save_data.insert(data_id.into(), data);
    }

    /// Calls `bincode::serialize` on the passed in data.
    /// Then sends that data into the `save` method, with the given data id.
    pub fn serialize_data(&mut self, data_id: impl Into<String>, data: &impl Serialize) {
        self.save(
            data_id,
            bincode::serialize(data).expect("Error serializing data!"),
        );
    }

    /// Reads the data as raw bytes at the given data id. Use `deserialize_data` for a streamlined way to read the data.
    pub fn read_data(&self, data_id: &str) -> Option<&Vec<u8>> {
        self.save_data.get(data_id)
    }

    /// Deserializes the data as the given type (via `bincode::deserialize`) at the given id. Will panic if the
    /// data is not properly serialized.
    pub fn deserialize_data<'a, T: Deserialize<'a>>(&'a self, data_id: &str) -> Option<T> {
        self.read_data(data_id)
            .map(|d| bincode::deserialize(d).expect("Error deserializing data!"))
    }
}

pub(super) fn register(app: &mut App) {
    saving::register(app);
    loading::register(app);
    player_loading::register(app);

    app.register_type::<EntityId>()
        .register_type::<SaveFileIdentifier>();
}
