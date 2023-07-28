//! Represents a one to one link
//!
//! Add this as a bevy resource by calling
//! [`create_one_to_one_registry`]

// use std::marker::PhantomData;
// use std::slice::Iter;

// use bevy::prelude::{App, Resource};

// use super::identifiable::Identifiable;
// use super::Registry;

// #[derive(Debug)]
// struct Mapping<V: Identifiable> {
//     key: V,

// }

// /// Represents a one to one link
// #[derive(Resource, Debug)]
// pub struct OneToOneRegistry<K: Identifiable, V: Identifiable> {
//     registry: Registry<V>,
//     _phantom: PhantomData<K>,
// }

// impl<K: Identifiable, V: Identifiable> Default for OneToOneRegistry<K, V> {
//     fn default() -> Self {
//         Self {
//             registry: Default::default(),
//             _phantom: Default::default(),
//         }
//     }
// }

// impl<K: Identifiable, V: Identifiable> OneToOneRegistry<K, V> {
//     /// Initializes a OneToOne relationship.
//     ///
//     /// You should use [`create_one_to_one_registry`] instead, unless you don't want this
//     /// added as a bevy resource.
//     pub fn new() -> Self {
//         Self::default()
//     }

//     pub fn insert(&self, key: &K, value: V) {
//         self.registry.register(item)
//     }

//     /// Gets the value a given key points to.
//     ///
//     /// Because this is a OneToOne relationship, multiple keys can point to the same value.
//     pub fn get_value(&self, key: &K) -> Option<&V> {
//         self.registry.try_from_numeric_id(key.id())
//     }

//     /// Iterates over all the values stored in this -- not the keys.
//     pub fn iter(&self) -> Iter<V> {
//         self.registry.iter()
//     }

//     /// Returns true if this registry contains an entry for that key
//     pub fn contains(&self, key: &K) -> bool {
//         self.registry.try_from_numeric_id(key.id()).is_some()
//     }
// }

// /// Initializes & adds the resource to bevy that can then be used in systems via `Res<OneToOneRegistry<K, V>>`
// pub fn create_one_to_one_registry<K: Identifiable + Sync + Send + 'static, V: Identifiable + Sync + Send + 'static>(app: &mut App) {
//     app.insert_resource(OneToOneRegistry::<K, V>::new());
// }
