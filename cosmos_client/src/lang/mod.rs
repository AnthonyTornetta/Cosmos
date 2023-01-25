pub mod load_langs;

use std::{fs, marker::PhantomData};

use bevy::{
    prelude::{App, Resource},
    utils::HashMap,
};
use cosmos_core::registry::identifiable::Identifiable;

#[derive(Resource)]
pub struct Lang<T: Identifiable + Send + Sync> {
    map: HashMap<u16, String>,
    id_map: HashMap<String, u16>,
    lang_contents: HashMap<String, String>,
    _phantom: PhantomData<T>,
}

fn load_data(lang_type: &str, lang_folder: &str, map: &mut HashMap<String, String>) {
    let path = format!("assets/lang/{lang_folder}/{lang_type}.lang");
    let str = fs::read_to_string(path.clone())
        .unwrap_or_else(|_| panic!("Error reading lang file @ '{path}'!"));

    for line in str
        .split('\n')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty() && !x.starts_with('#'))
    {
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

    /// Returns true if a record existed for this or not, false if not
    pub fn register(&mut self, item: &T) -> bool {
        match self.lang_contents.get(item.unlocalized_name()) {
            Some(name) => {
                self.map.insert(item.id(), name.clone());
                self.id_map
                    .insert(item.unlocalized_name().to_owned(), item.id());
                true
            }
            None => false,
        }
    }

    #[inline]
    pub fn get_name(&self, item: &T) -> Option<&String> {
        self.map.get(&item.id())
    }

    #[inline]
    pub fn get_name_from_id(&self, id: &str) -> Option<&String> {
        match self.id_map.get(id) {
            Some(id) => self.map.get(id),
            None => None,
        }
    }

    pub fn get_name_from_numeric_id(&self, id: u16) -> Option<&String> {
        self.map.get(&id)
    }
}

pub fn register(app: &mut App) {
    load_langs::register(app);
}
