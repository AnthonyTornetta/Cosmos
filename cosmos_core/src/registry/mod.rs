//! Handles the various types of registries you can use to register data.

pub mod identifiable;
pub mod many_to_one;

use bevy::prelude::{App, Resource};
use bevy::utils::HashMap;
use std::fmt;
use std::slice::Iter;

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
#[derive(Default, Resource)]
pub struct Registry<T: Identifiable + Sync + Send> {
    contents: Vec<T>,
    unlocalized_name_to_id: HashMap<String, u16>,
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
        self.unlocalized_name_to_id
            .insert(item.unlocalized_name().to_owned(), id);
        self.contents.push(item);
    }

    /// Iterates over every registered value
    pub fn iter(&self) -> Iter<T> {
        self.contents.iter()
    }
}

/// Initializes & adds the registry to bevy that can then be used in systems via `Res<Registry<T>>`
pub fn create_registry<T: Identifiable + Sync + Send + 'static>(app: &mut App) {
    app.insert_resource(Registry::<T>::new());
}
