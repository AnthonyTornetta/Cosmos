use bevy::utils::HashMap;

use self::item::Item;

pub mod item;

#[derive(Default)]
pub struct Items {
    items: Vec<Item>,
    items_to_string: HashMap<String, u16>,
}

impl Items {
    pub fn new() -> Self {
        Self::default()
    }

    /// Prefer to use `Self::block_from_id` in general, numeric IDs may change, unlocalized names should not
    pub fn item_from_numeric_id(&self, id: u16) -> &Item {
        &self.items[id as usize]
    }

    pub fn item_from_id(&self, id: &str) -> Option<&Item> {
        if let Some(num_id) = self.items_to_string.get(id) {
            Some(self.item_from_numeric_id(*num_id))
        } else {
            None
        }
    }

    pub fn register_block(&mut self, mut item: Item) {
        let id = self.items.len() as u16;
        item.set_numeric_id(id);
        self.items_to_string
            .insert(item.unlocalized_name().to_owned(), id);
        self.items.push(item);
    }
}
