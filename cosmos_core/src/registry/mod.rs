pub mod identifiable;
pub mod multi_registry;

use bevy::prelude::{App, Resource};
use bevy::utils::HashMap;
use std::slice::Iter;

use self::identifiable::Identifiable;

#[derive(Default, Resource)]
pub struct Registry<T: Identifiable + Sync + Send> {
    contents: Vec<T>,
    unlocalized_name_to_id: HashMap<String, u16>,
}

impl<T: Identifiable + Sync + Send> Registry<T> {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            unlocalized_name_to_id: HashMap::new(),
        }
    }

    /// Prefer to use `Self::from_id` in general, numeric IDs may change, unlocalized names should not
    #[inline]
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

    pub fn iter(&self) -> Iter<T> {
        self.contents.iter()
    }
}

pub fn create_registry<T: Identifiable + Sync + Send + 'static>(app: &mut App) {
    app.insert_resource(Registry::<T>::new());
}
