//! Handles the various types of registries you can use to register data.

pub mod identifiable;
pub mod many_to_one;
pub mod one_to_one;

use bevy::prelude::{resource_exists_and_changed, App, IntoSystemConfigs, Res, ResMut, Resource, Update};
use bevy::utils::HashMap;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::slice::{Iter, IterMut};
use std::sync::{Arc, RwLock, RwLockReadGuard};

use self::identifiable::Identifiable;

#[derive(Debug)]
/// This error will be returned if a link for a given unlocalized name could not be found.
///
/// Used in the `ManyToOne` registry.
pub enum AddLinkError {
    /// The unlocalized name specified could not be found.
    UnlocalizedNameNotFound {
        /// The unlocalized name passed in.
        name: String,
    },
}

impl fmt::Display for AddLinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            &Self::UnlocalizedNameNotFound { name } => {
                write!(f, "No link was found for the unlocalized name of {name}")
            }
        }
    }
}

impl std::error::Error for AddLinkError {}

/// Represents a bunch of values that are identifiable by their unlocalized name + numeric ids.
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Registry<T: Identifiable> {
    contents: Vec<T>,
    unlocalized_name_to_id: HashMap<String, u16>,
    /// Used for network syncing
    registry_name: String,
}

impl<T: Identifiable + Sync + Send> Registry<T> {
    /// Gets the unlocalized name for this registry - used for syncing registries from server -> client
    pub fn name(&self) -> &str {
        &self.registry_name
    }

    /// Initializes a Registry.
    ///
    /// You should use [`create_registry`] instead, unless you don't want this
    /// added as a bevy resource.
    pub fn new(registry_name: impl Into<String>) -> Self {
        Self {
            contents: Vec::new(),
            unlocalized_name_to_id: HashMap::new(),
            registry_name: registry_name.into(),
        }
    }

    /// Prefer to use `Self::from_id` in general, numeric IDs may change, unlocalized names should not
    ///
    /// This assumes the id has been registered, and will panic if it hasn't been
    #[inline]
    pub fn from_numeric_id(&self, id: u16) -> &T {
        &self.contents[id as usize]
    }

    /// Prefer to use `Self::from_id_mut` in general, numeric IDs may change, unlocalized names should not
    ///
    /// This assumes the id has been registered, and will panic if it hasn't been
    #[inline]
    pub fn from_numeric_id_mut(&mut self, id: u16) -> &mut T {
        &mut self.contents[id as usize]
    }

    /// Prefer to use `Self::from_id` in general, numeric IDs may change, unlocalized names should not
    ///
    /// Returns None if no item exists for this id
    #[inline]
    pub fn try_from_numeric_id(&self, id: u16) -> Option<&T> {
        self.contents.get(id as usize)
    }

    /// Prefer to use `Self::from_id_mut` in general, numeric IDs may change, unlocalized names should not
    ///
    /// Returns None if no item exists for this id
    #[inline]
    pub fn try_from_numeric_id_mut(&mut self, id: u16) -> Option<&mut T> {
        self.contents.get_mut(id as usize)
    }

    /// Gets the value that has been registered with that unlocalized name.
    ///
    /// Returns None if no value was found.
    pub fn from_id(&self, id: &str) -> Option<&T> {
        if let Some(num_id) = self.unlocalized_name_to_id.get(id) {
            Some(self.from_numeric_id(*num_id))
        } else {
            None
        }
    }

    /// Gets the value that has been registered with that unlocalized name.
    ///
    /// Returns None if no value was found.
    pub fn from_id_mut(&mut self, id: &str) -> Option<&mut T> {
        if let Some(num_id) = self.unlocalized_name_to_id.get(id) {
            Some(self.from_numeric_id_mut(*num_id))
        } else {
            None
        }
    }

    /// Adds an item to this registry.
    pub fn register(&mut self, mut item: T) {
        let id = self.contents.len() as u16;
        item.set_numeric_id(id);
        self.unlocalized_name_to_id.insert(item.unlocalized_name().to_owned(), id);
        self.contents.push(item);
    }

    /// Iterates over every registered value.
    pub fn iter(&self) -> Iter<T> {
        self.contents.iter()
    }

    /// Iterates over every registered value mutably.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.contents.iter_mut()
    }

    /// Returns true if an item with this unlocalized name exists & has been registered.
    pub fn contains(&self, unlocalized_name: &str) -> bool {
        self.unlocalized_name_to_id.contains_key(unlocalized_name)
    }

    /// Returns true if this registry contains nothing.
    pub fn is_empty(&self) -> bool {
        self.unlocalized_name_to_id.is_empty()
    }
}

/// Represents a bunch of values that are identifiable by their unlocalized name + numeric ids.
///
/// This is synced with its corresponding Registry<T> every frame when it's changed.
///
/// This is slower than a normal registry, but is usable between threads.
///
/// Any updates made to this will be overwritten whenever the Registry<T> changes, so don't change this
/// and expect anything to happen to the normal registry.
#[derive(Resource, Debug, Clone)]
pub struct ReadOnlyRegistry<T: Identifiable>(Arc<RwLock<Registry<T>>>);

impl<T: Identifiable + Sync + Send> ReadOnlyRegistry<T> {
    /// Initializes a Registry.
    ///
    /// You should use [`create_registry`] instead, unless you don't want this
    /// added as a bevy resource.
    pub fn new(registry_name: impl Into<String>) -> Self {
        Self(Arc::new(RwLock::new(Registry::new(registry_name))))
    }

    /// Takes a lock of the registry this encapsulates
    pub fn registry(&self) -> RwLockReadGuard<'_, Registry<T>> {
        self.0.as_ref().read().expect("Failed to lock registry")
    }
}

fn apply_changes<T: Identifiable + 'static>(registry: Res<Registry<T>>, mut mutex_registry: ResMut<ReadOnlyRegistry<T>>) {
    mutex_registry.0 = Arc::new(RwLock::new(registry.clone()));
}

/// Initializes & adds the registry to bevy that can then be used in systems via `Res<Registry<T>>`
pub fn create_registry<T: Identifiable + 'static>(app: &mut App, registry_name: impl Into<String> + Clone) {
    app.insert_resource(Registry::<T>::new(registry_name.clone()))
        .insert_resource(ReadOnlyRegistry::<T>::new(registry_name))
        .add_systems(Update, apply_changes::<T>.run_if(resource_exists_and_changed::<Registry<T>>));
}
