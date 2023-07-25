//! Represents a many to one link
//!
//! Add this as a bevy resource by calling
//! [`create_many_to_one_registry`]

use std::marker::PhantomData;

use bevy::prelude::{App, Resource};
use bevy::utils::hashbrown::hash_map::Values;
use bevy::utils::HashMap;

use super::identifiable::Identifiable;
use super::AddLinkError;

/// Represents a many to one link
#[derive(Resource, Default, Debug)]
pub struct ManyToOneRegistry<K: Identifiable + Sync + Send, V: Identifiable + Sync + Send> {
    values: HashMap<u16, V>,

    name_to_value_pointer: HashMap<String, u16>,
    /// Each value of pointers is a key of contents
    pointers: HashMap<u16, u16>,

    next_id: u16,

    _phantom: PhantomData<K>,
}

impl<K: Identifiable + Sync + Send, V: Identifiable + Sync + Send> ManyToOneRegistry<K, V> {
    /// Initializes a ManyToOne relationship.
    ///
    /// You should use [`create_many_to_one_registry`] instead, unless you don't want this
    /// added as a bevy resource.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            values: HashMap::default(),
            pointers: HashMap::default(),
            name_to_value_pointer: HashMap::default(),
            _phantom: PhantomData,
        }
    }

    /// Inserts a value into this relationship but does not link it to anything.
    ///
    /// Use [`ManyToOne::add_link`] to then link keys to this value.
    pub fn insert_value(&mut self, mut value: V) {
        let id = self.next_id;
        value.set_numeric_id(id);
        self.next_id += 1;

        self.name_to_value_pointer.insert(value.unlocalized_name().into(), id);
        self.values.insert(id, value);
    }

    /// Adds a link to the many to one relationship
    ///
    /// Will return an Ok result if the unlocalized_name already exists as a value in the registry.
    /// This means you have to call [`ManyToOne::insert_value`] first before using this.
    pub fn add_link(&mut self, key: &K, unlocalized_name: &str) -> Result<(), AddLinkError> {
        let ptr = *self
            .name_to_value_pointer
            .get(unlocalized_name)
            .ok_or_else(|| AddLinkError::UnlocalizedNameNotFound {
                name: unlocalized_name.to_owned(),
            })?;

        println!("Inserting {} : {unlocalized_name}", key.id());

        self.pointers.insert(key.id(), ptr);

        Ok(())
    }

    /// Gets the value a given key points to.
    ///
    /// Because this is a ManyToOne relationship, multiple keys can point to the same value.
    pub fn get_value(&self, key: &K) -> Option<&V> {
        self.pointers.get(&key.id()).map(|id| {
            self.values
                .get(id)
                .expect("ManyToOne pointers should always be valid, but this one wasn't.")
        })
    }

    /// Iterates over all the values stored in this -- not the keys.
    pub fn iter(&self) -> Values<'_, u16, V> {
        self.values.values()
    }

    /// Returns true if this registry contains an entry for that key
    pub fn contains(&self, key: &K) -> bool {
        self.pointers.contains_key(&key.id())
    }
}

/// Initializes & adds the resource to bevy that can then be used in systems via `Res<ManyToOneRegistry<K, V>>`
pub fn create_many_to_one_registry<K: Identifiable + Sync + Send + 'static, V: Identifiable + Sync + Send + 'static>(app: &mut App) {
    app.insert_resource(ManyToOneRegistry::<K, V>::new());
}
