pub mod identifiable;

use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use bevy::ecs::schedule::StateData;
use bevy::prelude::{App, Commands, EventWriter, ResMut, Resource, SystemSet};
use bevy::utils::HashMap;

use self::identifiable::Identifiable;

#[derive(Default, Resource)]
pub struct Registry<T: Identifiable + Sync + Send> {
    contents: Vec<T>,
    unlocalized_name_to_id: HashMap<String, u16>,
}

pub static AIR_BLOCK_ID: u16 = 0;

impl<T: Identifiable + Sync + Send> Registry<T> {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            unlocalized_name_to_id: HashMap::new(),
        }
    }

    /// Prefer to use `Self::from_id` in general, numeric IDs may change, unlocalized names should not
    pub fn from_numeric_id(&self, id: u16) -> &T {
        &self.contents[id as usize]
    }

    pub fn from_id(&self, id: &str) -> Option<&T> {
        if let Some(num_id) = self.unlocalized_name_to_id.get(id) {
            Some(self.from_numeric_id(*num_id))
        } else {
            None
        }
    }

    pub fn register(&mut self, mut item: T) {
        let id = self.contents.len() as u16;
        item.set_numeric_id(id);
        self.unlocalized_name_to_id
            .insert(item.unlocalized_name().to_owned(), id);
        self.contents.push(item);
    }
}

fn add_registry_resource<T: Identifiable + Sync + Send>(
    mut commands: Commands,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let loader_id = loading.register_loader(&mut start_writer);

    let mut registry = Registry::<T>::new();

    commands.insert_resource(registry);

    loading.finish_loading(loader_id, &mut end_writer);
}

pub fn register<T: StateData + Clone, K: Identifiable + Sync + Send>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
) {
    app.add_system_set(
        SystemSet::on_enter(pre_loading_state).with_system(add_registry_resource::<K>),
    );
}
