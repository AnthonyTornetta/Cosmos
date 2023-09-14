//! Handles the various types of registries you can use to register data.

pub mod identifiable;
pub mod many_to_one;
pub mod one_to_one;

use bevy::prelude::{resource_exists_and_changed, App, IntoSystemConfigs, Res, ResMut, Resource, Update};
use bevy::utils::HashMap;
use std::fmt;
use std::slice::Iter;
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
#[derive(Resource, Debug, Clone)]
pub struct Registry<T: Identifiable> {
    contents: Vec<T>,
    unlocalized_name_to_id: HashMap<String, u16>,
}

impl<T: Identifiable> Default for Registry<T> {
    fn default() -> Self {
        Self {
            contents: Vec::new(),
            unlocalized_name_to_id: Default::default(),
        }
    }
}

impl<T: Identifiable + Sync + Send> Registry<T> {
    /// Initializes a Registry.
    ///
    /// You should use [`create_registry`] instead, unless you don't want this
    /// added as a bevy resource.
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
            unlocalized_name_to_id: HashMap::new(),
        }
    }

    /// Prefer to use `Self::from_id` in general, numeric IDs may change, unlocalized names should not
    ///
    /// This assumes the id has been registered, and will panic if it hasn't been
    #[inline]
    pub fn from_numeric_id(&self, id: u16) -> &T {
        &self.contents[id as usize]
    }

    /// Prefer to use `Self::from_id` in general, numeric IDs may change, unlocalized names should not
    ///
    /// Returns None if no item exists for this id
    #[inline]
    pub fn try_from_numeric_id(&self, id: u16) -> Option<&T> {
        self.contents.get(id as usize)
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

    /// Adds an item to this registry
    pub fn register(&mut self, mut item: T) {
        let id = self.contents.len() as u16;
        item.set_numeric_id(id);
        self.unlocalized_name_to_id.insert(item.unlocalized_name().to_owned(), id);
        self.contents.push(item);
    }

    /// Iterates over every registered value
    pub fn iter(&self) -> Iter<T> {
        self.contents.iter()
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

impl<T: Identifiable> Default for ReadOnlyRegistry<T> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(Registry::default())))
    }
}

impl<T: Identifiable + Sync + Send> ReadOnlyRegistry<T> {
    /// Initializes a Registry.
    ///
    /// You should use [`create_registry`] instead, unless you don't want this
    /// added as a bevy resource.
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(Registry::new())))
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
pub fn create_registry<T: Identifiable + 'static>(app: &mut App) {
    app.insert_resource(Registry::<T>::new())
        .insert_resource(ReadOnlyRegistry::<T>::new())
        .add_systems(Update, apply_changes::<T>.run_if(resource_exists_and_changed::<Registry<T>>()));
}
