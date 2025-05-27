//! Items are something that represent something that can be stored in inventories.

pub mod item_category;
pub mod items;
pub mod physical_item;

use bevy::{prelude::App, prelude::States};

use crate::registry::identifiable::Identifiable;

#[derive(Debug, Clone)]
/// An item represents something that can be stored in inventories.
pub struct Item {
    unlocalized_name: String,
    numeric_id: u16,
    max_stack_size: u16,
    category: Option<String>,
}

impl Identifiable for Item {
    #[inline]
    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    #[inline]
    fn id(&self) -> u16 {
        self.numeric_id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.numeric_id = id;
    }
}

/// The max stack size for items, should load this from config file in future
pub const DEFAULT_MAX_STACK_SIZE: u16 = 999;

impl Item {
    /// Creates an item
    pub fn new(unlocalized_name: impl Into<String>, max_stack_size: u16, category: Option<String>) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            numeric_id: 0, // this will get set when this item is registered
            max_stack_size,
            category,
        }
    }

    /// Returns the max stack size for this item
    pub fn max_stack_size(&self) -> u16 {
        self.max_stack_size
    }

    /// If this item has a category, this returns that category as `Some` category.
    pub fn category(&self) -> Option<&str> {
        self.category.as_ref().map(|x| x.as_str())
    }
}

pub struct ItemBuilder {
    unlocalized_name: String,
    max_stack_size: u16,
    category: Option<String>,
}

impl ItemBuilder {
    pub fn new(unlocalized_name: impl Into<String>) -> Self {
        Self {
            unlocalized_name: unlocalized_name.into(),
            category: None,
            max_stack_size: DEFAULT_MAX_STACK_SIZE,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());

        self
    }

    pub fn with_stack_size(mut self, stack_size: u16) -> Self {
        self.max_stack_size = stack_size;

        self
    }

    pub fn create(self) -> Item {
        Item::new(self.unlocalized_name, self.max_stack_size, self.category)
    }
}

pub(super) fn register<T: States>(app: &mut App, loading_state: T) {
    items::register(app, loading_state);
    physical_item::register(app);
    item_category::register(app);
}
