pub mod items;

use bevy::prelude::App;

use crate::registry::identifiable::Identifiable;

pub struct Item {
    unlocalized_name: String,
    numeric_id: u16,
    max_stack_size: u16,
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

pub const DEFAULT_MAX_STACK_SIZE: u16 = 999;

impl Item {
    pub fn new(unlocalized_name: String, max_stack_size: u16) -> Self {
        Self {
            unlocalized_name,
            numeric_id: 0, // this will get set when this item is registered
            max_stack_size,
        }
    }

    pub fn max_stack_size(&self) -> u16 {
        self.max_stack_size
    }
}

pub fn register(app: &mut App) {
    items::register(app);
}
