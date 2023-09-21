//! An inventory consists of a list of ItemStacks
//!
//! These ItemStacks can be modified freely. An inventory is owned by an entity.

use bevy::{
    prelude::{App, Component},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{item::Item, registry::identifiable::Identifiable};

use self::itemstack::ItemStack;

pub mod itemstack;
pub mod netty;

// TODO
// pub enum InventoryType {
//     BulkInventory,   // These inventories are not organizable by the player
//     NormalInventory, // These inventories are organizable by the player
// }

#[derive(Default, Component, Serialize, Deserialize, Debug, Reflect, Clone)]
/// A collection of ItemStacks, organized into slots
pub struct Inventory {
    items: Vec<Option<ItemStack>>,
}

impl Inventory {
    /// Creates an empty inventory with that number of slots
    pub fn new(n_slots: usize) -> Self {
        let mut items = Vec::with_capacity(n_slots);

        for _ in 0..n_slots {
            items.push(None);
        }

        Self { items }
    }

    /// The number of slots this inventory contains
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// If this inventory contains no items
    ///
    /// **Note:** An inventory may be empty but have a non-zero `len()`!
    pub fn is_empty(&self) -> bool {
        self.items.iter().any(|x| x.is_some())
    }

    /// Swaps the contents of two inventory slots in the same inventory.
    ///
    /// Returns Ok if both slots were within the bounds of the inventory, Err if either was not
    pub fn self_swap_slots(&mut self, slot_a: usize, slot_b: usize) -> Result<(), ()> {
        if !(slot_a < self.items.len() && slot_b < self.items.len()) {
            return Err(());
        }

        self.items.swap(slot_a, slot_b);

        Ok(())
    }

    /// Swaps the contents of two inventory slots in two different inventories
    ///
    /// Returns Ok if both slots were within the bounds of their inventories, Err if either was not
    pub fn swap_slots(&mut self, this_slot: usize, other: &mut Inventory, their_slot: usize) -> Result<(), ()> {
        if !(this_slot < self.items.len() && their_slot < other.len()) {
            return Err(());
        }

        std::mem::swap(&mut self.items[this_slot], &mut other.items[their_slot]);

        Ok(())
    }

    /// Returns the overflow that could not fit
    pub fn insert(&mut self, item: &Item, mut quantity: u16) -> u16 {
        // Search for existing stacks, if none found that make new one(s)

        for is in &mut self.items.iter_mut().flatten().filter(|x| x.item_id() == item.id()) {
            quantity = is.increase_quantity(quantity);

            if quantity == 0 {
                return 0;
            }
        }

        // no suitable locations found with pre-existing stacks of that item, make new ones

        for i in 0..self.items.len() {
            if self.items[i].is_none() {
                let mut is = ItemStack::new(item);
                quantity = is.increase_quantity(quantity);

                self.items[i] = Some(is);

                if quantity == 0 {
                    return 0;
                }
            }
        }

        // if any amount is left over, it will be represented in the quantity variable

        quantity
    }

    /// Returns the ItemStack at that slot
    pub fn itemstack_at(&self, slot: usize) -> Option<&ItemStack> {
        self.items[slot].as_ref()
    }

    /// Returns the overflow quantity
    pub fn decrease_quantity_at(&mut self, slot: usize, amount: u16) -> u16 {
        if let Some(is) = &mut self.items[slot] {
            let res = is.decrease_quantity(amount);

            if is.is_empty() {
                self.items[slot] = None;
            }

            res
        } else {
            amount
        }
    }

    /// Returns the overflow quantity
    pub fn increase_quantity_at(&mut self, slot: usize, amount: u16) -> u16 {
        if let Some(slot) = &mut self.items[slot] {
            slot.increase_quantity(amount)
        } else {
            amount
        }
    }

    /// Sets the ItemStack stored at that slot number. Will overwrite any previous stack
    pub fn set_itemstack_at(&mut self, slot: usize, item_stack: Option<ItemStack>) {
        self.items[slot] = item_stack;
    }

    /// Inserts the items & quantity at that slot. Returns the number of items left over, or the full
    /// quantity of items if that slot doesn't represent that item.
    pub fn insert_at(&mut self, slot: usize, item: &Item, quantity: u16) -> u16 {
        if let Some(slot) = &mut self.items[slot] {
            if slot.item_id() != item.id() {
                quantity
            } else {
                slot.increase_quantity(quantity)
            }
        } else {
            self.items[slot] = Some(ItemStack::with_quantity(item, quantity));

            0
        }
    }

    /// Calculates the number of that specific item in this inventory.
    pub fn quantity_of(&self, item: &Item) -> usize {
        self.items
            .iter()
            .filter_map(|x| x.as_ref())
            .filter(|x| x.item_id() == item.id())
            .map(|x| x.quantity() as usize)
            .sum()
    }

    /// Iterates over every slot in the inventory.
    pub fn iter(&self) -> std::slice::Iter<'_, std::option::Option<ItemStack>> {
        self.items.iter()
    }
}

pub(super) fn register(app: &mut App) {
    itemstack::register(app);
    app.register_type::<Inventory>();
}
