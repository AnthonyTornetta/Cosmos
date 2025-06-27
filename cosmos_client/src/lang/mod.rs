//! Used to get human-readalbe & localized text for various identifiable types.

mod load_langs;

use std::{fs, marker::PhantomData};

use bevy::{
    log::warn,
    platform::collections::HashMap,
    prelude::{App, Resource},
};
use cosmos_core::{block::Block, item::Item, registry::identifiable::Identifiable};

#[derive(Resource)]
/// Used to get the human-readable + localized text to display for identifiable types
pub struct Lang<T: Identifiable + Send + Sync> {
    map: HashMap<u16, String>,
    id_map: HashMap<String, u16>,
    lang_contents: HashMap<String, String>,
    _phantom: PhantomData<T>,
}

fn load_data(lang_type: &str, lang_folder: &str, map: &mut HashMap<String, String>) {
    let path = format!("assets/cosmos/lang/{lang_folder}/{lang_type}.lang");
    let str = fs::read_to_string(path.clone()).unwrap_or_else(|_| panic!("Error reading lang file @ '{path}'!"));

    for line in str.split('\n').map(|x| x.trim()).filter(|x| !x.is_empty() && !x.starts_with('#')) {
        let split: Vec<&str> = line.split('=').collect();

        if split.len() == 1 {
            panic!("Error parsing lang file {path}. Invalid line - {line} (missing = sign)");
        }

        if !map.contains_key(split[0]) {
            map.insert(split[0].to_owned(), split[1..].concat());
        }
    }
}

impl<T: Identifiable + Send + Sync> Lang<T> {
    /// Creates a language instance for from a specific file.
    ///
    /// * `lang_type` The language identifier, such as en_us
    /// * `read_from` These are the files that should be read from for the language data. These should be sorted in order of importance - data found in the file N will override data found files N + X.
    pub fn new(lang_type: &str, read_from: Vec<&str>) -> Self {
        let mut lang_contents = HashMap::new();

        for fallback in read_from {
            load_data(lang_type, fallback, &mut lang_contents);
        }

        Self {
            lang_contents,
            map: HashMap::default(),
            _phantom: PhantomData,
            id_map: HashMap::default(),
        }
    }

    /// This is used to add a usable entry
    ///
    /// Returns true if a record existed for this or not, false if not
    pub fn register(&mut self, item: &T) -> bool {
        match self.lang_contents.get(item.unlocalized_name()) {
            Some(name) => {
                self.map.insert(item.id(), name.clone());
                self.id_map.insert(item.unlocalized_name().to_owned(), item.id());
                true
            }
            None => {
                warn!("Missing lang file entry for {}", item.unlocalized_name());
                false
            }
        }
    }

    #[inline]
    /// Gets the text for this specific entry
    ///
    /// Make sure `register(item)` was called first!
    pub fn get_name(&self, item: &T) -> Option<&str> {
        self.get_name_from_numeric_id(item.id())
    }

    #[inline]
    /// Gets the text for this specific entry
    ///
    /// Make sure `register(item)` was called first!
    pub fn get_name_or_unlocalized<'a>(&'a self, item: &'a T) -> &'a str {
        self.get_name_from_numeric_id(item.id()).unwrap_or(item.unlocalized_name())
    }

    #[inline]
    /// Gets the text for an entry based off its unlocalized name.
    ///
    /// Make sure `register(item)` was called first!
    pub fn get_name_from_id(&self, id: &str) -> Option<&str> {
        match self.id_map.get(id) {
            Some(id) => self.map.get(id).map(String::as_str),
            None => None,
        }
    }

    #[inline]
    /// Gets the text for this specific numerical id.
    ///
    /// Make sure `register(item)` was called first!
    pub fn get_name_from_numeric_id(&self, id: u16) -> Option<&str> {
        self.map.get(&id).map(|x| x.as_str())
    }
}

/// Loads entries for this type from the given `read_from` lang file entries. The order
/// dictates the priority given to each file.
pub fn register_lang<T: Identifiable>(app: &mut App, read_from: Vec<&'static str>) {
    load_langs::register::<T>(app, read_from);
}

pub(super) fn register(app: &mut App) {
    register_lang::<Block>(app, vec!["blocks"]);
    register_lang::<Item>(app, vec!["blocks", "items"]);
}
