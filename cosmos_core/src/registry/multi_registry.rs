use std::marker::PhantomData;
use std::slice::Iter;

use bevy::prelude::{App, Resource};
use bevy::utils::HashMap;

use super::identifiable::Identifiable;

#[derive(Resource)]
pub struct MultiRegistry<K: Identifiable + Sync + Send, V: Identifiable + Sync + Send> {
    contents: Vec<V>,
    pointers: HashMap<u16, usize>,

    _phantom: PhantomData<K>,
}

pub static AIR_BLOCK_ID: u16 = 0;

impl<K: Identifiable + Sync + Send, V: Identifiable + Sync + Send> MultiRegistry<K, V> {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            pointers: HashMap::new(),
            _phantom: PhantomData::default(),
        }
    }

    pub fn insert_value(&mut self, value: V) {
        self.contents.push(value);
    }

    pub fn add_link(&mut self, key: &K, unlocalized_name: &str) -> Result<(), ()> {
        for (i, item) in self.contents.iter().enumerate() {
            if item.unlocalized_name() == unlocalized_name {
                self.pointers.insert(key.id(), i);

                return Ok(());
            }
        }

        Err(())
    }

    pub fn get_value(&self, key: &K) -> Option<&V> {
        if let Some(index) = self.pointers.get(&key.id()) {
            Some(&self.contents[*index])
        } else {
            None
        }
    }

    pub fn iter(&self) -> Iter<V> {
        self.contents.iter()
    }
}

pub fn create_multi_registry<
    K: Identifiable + Sync + Send + 'static,
    V: Identifiable + Sync + Send + 'static,
>(
    app: &mut App,
) {
    app.insert_resource(MultiRegistry::<K, V>::new());
}
