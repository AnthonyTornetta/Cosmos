//! An inventory consists of a list of ItemStacks
//!
//! These ItemStacks can be modified freely. An inventory is owned by an entity.

use std::ops::Range;

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

/// Represents some sort of error that occurred
#[derive(Debug)]
pub enum InventoryError {
    /// A slot outside the range of this inventory was given
    InvalidSlot(usize),
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::InvalidSlot(slot) => f.write_str(&format!("Invalid slot {}", slot)),
        }
    }
}

#[derive(Default, Component, Serialize, Deserialize, Debug, Reflect, Clone)]
/// A collection of ItemStacks, organized into slots
pub struct Inventory {
    items: Vec<Option<ItemStack>>,
    priority_slots: Option<Range<usize>>,
}

impl Inventory {
    /// Creates an empty inventory with that number of slots
    pub fn new(n_slots: usize, priority_slots: Option<Range<usize>>) -> Self {
        let mut items = Vec::with_capacity(n_slots);

        for _ in 0..n_slots {
            items.push(None);
        }

        Self { items, priority_slots }
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
    pub fn self_swap_slots(&mut self, slot_a: usize, slot_b: usize) -> Result<(), InventoryError> {
        if slot_a >= self.items.len() {
            return Err(InventoryError::InvalidSlot(slot_a));
        }
        if slot_b >= self.items.len() {
            return Err(InventoryError::InvalidSlot(slot_b));
        }

        self.items.swap(slot_a, slot_b);

        Ok(())
    }

    /// Swaps the contents of two inventory slots in two different inventories
    ///
    /// Returns Ok if both slots were within the bounds of their inventories, Err if either was not
    pub fn swap_slots(&mut self, this_slot: usize, other: &mut Inventory, other_slot: usize) -> Result<(), InventoryError> {
        if this_slot >= self.items.len() {
            return Err(InventoryError::InvalidSlot(this_slot));
        }
        if other_slot >= other.len() {
            return Err(InventoryError::InvalidSlot(other_slot));
        }

        std::mem::swap(&mut self.items[this_slot], &mut other.items[other_slot]);

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

    /// Returns the ItemStack at that slot
    pub fn mut_itemstack_at(&mut self, slot: usize) -> Option<&mut ItemStack> {
        self.items[slot].as_mut()
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
        self.insert_raw_at(slot, item.id(), item.max_stack_size(), quantity)
    }

    /// Inserts the items & quantity at that slot. Returns the number of items left over, or the full
    /// quantity of items if that slot doesn't represent that item.
    fn insert_raw_at(&mut self, slot: usize, item_id: u16, max_stack_size: u16, quantity: u16) -> u16 {
        if let Some(slot) = &mut self.items[slot] {
            if slot.item_id() != item_id {
                quantity
            } else {
                slot.increase_quantity(quantity)
            }
        } else {
            self.items[slot] = Some(ItemStack::raw_with_quantity(item_id, max_stack_size, quantity));

            0
        }
    }

    /// Moves an item around an inventory to auto sort it
    pub fn auto_move(&mut self, slot: usize) -> Result<(), InventoryError> {
        if slot >= self.items.len() {
            return Err(InventoryError::InvalidSlot(slot));
        }

        let Some(mut item_stack) = self.itemstack_at(slot).cloned() else {
            return Ok(());
        };

        if let Some(priority_slots) = self.priority_slots.clone() {
            if !priority_slots.contains(&slot) {
                // attempt to move to priority slots first
                for slot in priority_slots {
                    let left_over = self.insert_raw_at(slot, item_stack.item_id(), item_stack.max_stack_size(), item_stack.quantity());

                    item_stack.set_quantity(left_over);

                    if item_stack.quantity() == 0 {
                        break;
                    }
                }
            }
        }

        let n = self.items.len();
        let priority_slots = self.priority_slots.clone();

        let slot_not_priority_slot = |x: &usize| priority_slots.clone().map(|range| !range.contains(x)).unwrap_or(true);

        for slot in (0..n).filter(|&x| x != slot).filter(slot_not_priority_slot) {
            if item_stack.quantity() == 0 {
                break;
            }

            let left_over = self.insert_raw_at(slot, item_stack.item_id(), item_stack.max_stack_size(), item_stack.quantity());

            item_stack.set_quantity(left_over);
        }

        if item_stack.quantity() != 0 {
            self.set_itemstack_at(slot, Some(item_stack));
        } else {
            self.set_itemstack_at(slot, None);
        }

        Ok(())
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
