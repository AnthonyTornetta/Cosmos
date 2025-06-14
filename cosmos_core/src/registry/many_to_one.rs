//! Represents a many to one link
//!
//! Add this as a bevy resource by calling
//! [`create_many_to_one_registry`]

use std::collections::hash_map::Values;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock, RwLockReadGuard};

use bevy::prelude::*;

use super::AddLinkError;
use super::identifiable::Identifiable;
use std::collections::HashMap;

/// Represents a many to one link
#[derive(Resource, Default, Debug, Clone)]
pub struct ManyToOneRegistry<K: Identifiable, V: Identifiable> {
    values: HashMap<u16, V>,

    name_to_value_pointer: HashMap<String, u16>,
    /// Each value of pointers is a key of contents
    pointers: HashMap<u16, u16>,

    next_id: u16,

    _phantom: PhantomData<K>,
}

impl<K: Identifiable, V: Identifiable> ManyToOneRegistry<K, V> {
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

/// This is synced with its corresponding ManyToOneRegistry<T> every frame when it's changed.
///
/// This is slower than a normal registry, but is usable between threads.
///
/// Any updates made to this will be overwritten whenever the ManyToOneRegistry<T> changes, so don't change this
/// and expect anything to happen to the normal registry.
#[derive(Resource, Debug, Clone)]
pub struct ReadOnlyManyToOneRegistry<K: Identifiable, V: Identifiable>(Arc<RwLock<ManyToOneRegistry<K, V>>>);

impl<K: Identifiable, V: Identifiable> Default for ReadOnlyManyToOneRegistry<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Identifiable, V: Identifiable> ReadOnlyManyToOneRegistry<K, V> {
    /// Initializes a Registry.
    ///
    /// You should use [`create_registry`] instead, unless you don't want this
    /// added as a bevy resource.
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(ManyToOneRegistry::new())))
    }

    /// Takes a lock of the registry this encapsulates
    pub fn registry(&self) -> RwLockReadGuard<'_, ManyToOneRegistry<K, V>> {
        self.0.as_ref().read().expect("Failed to lock registry")
    }
}

fn apply_changes<K: Identifiable, V: Identifiable>(
    registry: Res<ManyToOneRegistry<K, V>>,
    mut mutex_registry: ResMut<ReadOnlyManyToOneRegistry<K, V>>,
) {
    mutex_registry.0 = Arc::new(RwLock::new(registry.clone()));
}

/// Initializes & adds the resource to bevy that can then be used in systems via `Res<ManyToOneRegistry<K, V>>`
pub fn create_many_to_one_registry<K: Identifiable + 'static, V: Identifiable + 'static>(app: &mut App) {
    app.insert_resource(ManyToOneRegistry::<K, V>::new())
        .insert_resource(ReadOnlyManyToOneRegistry::<K, V>::new())
        .add_systems(
            Update,
            apply_changes::<K, V>
                .ambiguous_with_all() // This function will only ever run if the registry is changed, which should never happen.
                .run_if(resource_exists_and_changed::<ManyToOneRegistry<K, V>>),
        );
}
