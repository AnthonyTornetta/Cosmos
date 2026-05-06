//! Used for mapping `numeric id` -> `string id` so that serialized data stays consistent across
//! numeric ID reordering.

use bevy::{platform::collections::HashMap, prelude::*};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    inventory::{Inventory, itemstack::ItemStack},
    item::Item,
    netty::sync::IdentifiableComponent,
    prelude::Structure,
    registry::{Registry, identifiable::Identifiable},
};

#[derive(Serialize, Deserialize, Reflect, Clone, Debug, Component)]
/// Used for mapping `numeric id` -> `string id` so that serialized data stays consistent across
/// numeric ID reordering.
pub struct Palette<T: Identifiable + std::fmt::Debug> {
    mappings: HashMap<u16, String>,
    #[serde(skip)]
    cache: HashMap<u16, T>,
}

impl IdentifiableComponent for Palette<Block> {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:palette_block"
    }
}

impl IdentifiableComponent for Palette<Item> {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:palette_item"
    }
}

impl<T: Identifiable + std::fmt::Debug> Default for Palette<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Palette<Item> {
    /// Creates a new [`Palette<Item>`] for these itemstacks
    pub fn new_from_itemstacks<'a, I: Iterator<Item = &'a ItemStack>>(itemstacks: I, items: &Registry<Item>) -> Self {
        Self::new_from_items(itemstacks.map(|x| items.from_numeric_id(x.item_id())))
    }

    /// Creates a new [`Palette<Item>`] for these items
    pub fn new_from_items<'a, I: Iterator<Item = &'a Item>>(items: I) -> Self {
        Self::new(
            items
                .unique_by(|c| c.id())
                .map(|x| (x.id(), x.unlocalized_name().to_owned()))
                .collect::<HashMap<u16, String>>(),
        )
    }

    /// Creates a new [`Palette<Item>`] for this inventory
    pub fn new_from_inventory(inventory: &Inventory, items: &Registry<Item>) -> Self {
        Self::new_from_itemstacks(inventory.iter().flatten(), items)
    }
}

impl Palette<Block> {
    /// Creates a new [`Palette<Block>`] for this structure
    pub fn new_from_structure(structure: &Structure, blocks: &Registry<Block>) -> Self {
        Self::new(
            structure
                .all_blocks_iter(false)
                .map(|c| structure.block_id_at(c))
                .unique()
                .map(|x| (x, blocks.from_numeric_id(x).unlocalized_name().to_owned()))
                .collect::<HashMap<u16, String>>(),
        )
    }
}

impl<T: Identifiable + std::fmt::Debug> Palette<T> {
    /// Creates a new generic [`Palette`] with these mappigns
    pub fn new(mappings: HashMap<u16, String>) -> Self {
        Self {
            mappings,
            cache: Default::default(),
        }
    }

    /// Checks if this id exists within this
    pub fn exists(&self, id: u16) -> bool {
        self.mappings.contains_key(&id)
    }

    /// Gets the stored unlocalized name for this id
    pub fn get(&self, id: u16) -> Option<&String> {
        self.mappings.get(&id)
    }

    /// Pre-computes all `numeric id` -> `T` combinations. You don't need to manually call this, as
    /// it is automatically computed when [`Self::get_cached`] is called.
    fn compute_cache(&mut self, registry: &Registry<T>) -> Option<Vec<String>> {
        let mut missing: Vec<String> = vec![];
        for (id, name) in self.mappings.iter() {
            let Some(item) = registry.from_id(name) else {
                missing.push(name.to_owned());
                continue;
            };

            self.cache.insert(*id, item.clone());
        }

        if !missing.is_empty() {
            return Some(missing);
        }
        None
    }

    /// Gets the `T` for this id from a pre-computed cache of mappings. If the cache has not yet
    /// been computed, it is computed now.
    pub fn get_cached(&mut self, id: u16, registry: &Registry<T>) -> Option<&T> {
        if self.cache.is_empty()
            && let Some(missing) = self.compute_cache(registry)
        {
            error!("Invalid ids: {missing:?} in palette {self:?}");
        }

        self.cache.get(&id)
    }

    /// Sets the mapping for this numeric id to be this unlocalized name
    pub fn set(&mut self, id: u16, str_id: String) {
        self.mappings.insert(id, str_id);
    }
}
