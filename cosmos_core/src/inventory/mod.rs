//! An inventory consists of a list of ItemStacks
//!
//! These ItemStacks can be modified freely. An inventory is owned by an entity.

use std::ops::Range;

use bevy::{
    prelude::{App, Component, Deref, DerefMut},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{item::Item, registry::identifiable::Identifiable};

use self::itemstack::ItemStack;

pub mod held_item_slot;
pub mod itemstack;
pub mod netty;

// TODO
// pub enum InventoryType {
//     BulkInventory,   // These inventories are not organizable by the player
//     NormalInventory, // These inventories are organizable by the player
// }

#[derive(Component, DerefMut, Deref, Debug, Serialize, Deserialize, Clone, Reflect)]
/// This represents the itemstack the player is currently holding while moving items around in their inventory.
///
/// There should only ever be one HeldItemStack per player, and on the client only one or zero HeldItemStacks will ever exist at a time.
///
/// # THIS BEHAVES DIFFERENTLY ON THE CLIENT & SERVER
/// ### Client
/// On the client, this is attached to a GUI element holding the drawn item the player is moving w/ their cursor.
///
/// ### Server
/// On the server, this is attached directly to the player.
pub struct HeldItemStack(pub ItemStack);

/// Represents some sort of error that occurred
#[derive(Debug)]
pub enum InventorySlotError {
    /// A slot outside the range of this inventory was given
    InvalidSlot(usize),
}

impl std::fmt::Display for InventorySlotError {
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
    name: String,
}

impl Inventory {
    /// Creates an empty inventory with that number of slots
    pub fn new(name: impl Into<String>, n_slots: usize, priority_slots: Option<Range<usize>>) -> Self {
        let mut items = Vec::with_capacity(n_slots);

        for _ in 0..n_slots {
            items.push(None);
        }

        Self {
            items,
            priority_slots,
            name: name.into(),
        }
    }

    /// Returns the name of this inventory
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name of this inventory
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Returns the range of priority slots if this inventory has any
    pub fn priority_slots(&self) -> Option<Range<usize>> {
        self.priority_slots.clone()
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
    pub fn self_swap_slots(&mut self, slot_a: usize, slot_b: usize) -> Result<(), InventorySlotError> {
        if slot_a >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(slot_a));
        }
        if slot_b >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(slot_b));
        }

        self.items.swap(slot_a, slot_b);

        Ok(())
    }

    /// Swaps the contents of two inventory slots in two different inventories
    ///
    /// Returns Ok if both slots were within the bounds of their inventories, Err if either was not
    pub fn swap_slots(&mut self, this_slot: usize, other: &mut Inventory, other_slot: usize) -> Result<(), InventorySlotError> {
        if this_slot >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(this_slot));
        }
        if other_slot >= other.len() {
            return Err(InventorySlotError::InvalidSlot(other_slot));
        }

        std::mem::swap(&mut self.items[this_slot], &mut other.items[other_slot]);

        Ok(())
    }

    /// Returns true if there is enough space in this inventory to insert this itemstack.
    pub fn can_insert_itemstack(&self, itemstack: &ItemStack) -> bool {
        self.can_insert_raw(itemstack.item_id(), itemstack.max_stack_size(), itemstack.quantity())
    }

    /// Returns true if there is enough space in this inventory to insert an item of this quantity.
    pub fn can_insert(&self, item: &Item, quantity: u16) -> bool {
        self.can_insert_raw(item.id(), item.max_stack_size(), quantity)
    }
    /// Returns (the overflow that could not fit and the slot
    pub fn can_insert_raw(&self, item_id: u16, max_stack_size: u16, mut quantity: u16) -> bool {
        for is in &mut self.items.iter().flatten().filter(|x| x.item_id() == item_id) {
            let delta = max_stack_size - is.quantity();
            if delta >= quantity {
                return true;
            }

            quantity -= delta;
        }

        // no suitable locations found with pre-existing stacks of that item, check for new ones

        for _ in self.items.iter().filter(|x| x.is_none()) {
            if max_stack_size >= quantity {
                return true;
            }

            quantity -= max_stack_size;
        }

        false
    }

    /// Returns the overflow that could not fit
    pub fn insert_itemstack(&mut self, itemstack: &ItemStack) -> u16 {
        self.insert_raw(itemstack.item_id(), itemstack.max_stack_size(), itemstack.quantity())
    }

    /// Returns (the overflow that could not fit and the slot
    pub fn insert(&mut self, item: &Item, quantity: u16) -> u16 {
        self.insert_raw(item.id(), item.max_stack_size(), quantity)
    }

    /// Returns the overflow that could not fit
    fn insert_raw(&mut self, item_id: u16, max_stack_size: u16, mut quantity: u16) -> u16 {
        // Search for existing stacks, if none found that make new one(s)

        for is in &mut self.items.iter_mut().flatten().filter(|x| x.item_id() == item_id) {
            quantity = is.increase_quantity(quantity);

            if quantity == 0 {
                return 0;
            }
        }

        // no suitable locations found with pre-existing stacks of that item, make new ones

        for i in 0..self.items.len() {
            if self.items[i].is_none() {
                let mut is = ItemStack::raw_with_quantity(item_id, max_stack_size, 0);
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
    pub fn insert_item_at(&mut self, slot: usize, item: &Item, quantity: u16) -> u16 {
        self.insert_raw_at(slot, item.id(), item.max_stack_size(), quantity)
    }

    /// Inserts the items & quantity at that slot. Returns the number of items left over, or the full
    /// quantity of items if that slot doesn't represent that item.
    pub fn insert_item_stack_at(&mut self, slot: usize, itemstack: &ItemStack) -> u16 {
        self.insert_raw_at(slot, itemstack.item_id(), itemstack.max_stack_size(), itemstack.quantity())
    }

    /// Removes an itemstack at that slot and replaces it with `None`. Returns the itemstack previously in that slot
    pub fn remove_itemstack_at(&mut self, slot: usize) -> Option<ItemStack> {
        self.items[slot].take()
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
    pub fn auto_move(&mut self, slot: usize, amount: u16) -> Result<(), InventorySlotError> {
        if slot >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(slot));
        }

        let Some(mut item_stack) = self.itemstack_at(slot).cloned() else {
            return Ok(());
        };

        let final_left_over = if amount < item_stack.quantity() {
            let res = item_stack.quantity() - amount;

            item_stack.set_quantity(amount);

            res
        } else {
            0
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

        item_stack.set_quantity(item_stack.quantity() + final_left_over);

        if item_stack.quantity() != 0 {
            self.set_itemstack_at(slot, Some(item_stack));
        } else {
            self.set_itemstack_at(slot, None);
        }

        Ok(())
    }

    /// A quick way of comparing two different slots to see if they contain the same item
    pub fn can_move_itemstack_to(&self, is: &ItemStack, slot: usize) -> bool {
        self.itemstack_at(slot).map(|x| x.is_same_as(is)).unwrap_or(false)
    }

    /// A quick way of comparing two different slots to see if they contain the same item
    pub fn is_same_itemstack_as(&self, self_slot: usize, other_inventory: &Self, other_slot: usize) -> bool {
        let is_1 = self.itemstack_at(self_slot);
        let is_2 = other_inventory.itemstack_at(other_slot);

        if is_1.is_none() && is_2.is_none() {
            true
        } else if let Some(is_1) = is_1 {
            if let Some(is_2) = is_2 {
                is_1.is_same_as(is_2)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Moves an item from slot `from` to slot `to`.
    ///
    /// This will respect stack sizes, and returns the "left over" amount in the slot it was moved from.
    pub fn self_move_itemstack(&mut self, from: usize, to: usize, max_quantity: u16) -> Result<u16, InventorySlotError> {
        if from >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(from));
        }
        if to >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(to));
        }

        if from == to {
            return Ok(0);
        }

        let is = self.itemstack_at(from);
        let Some(is) = is else {
            return Ok(0);
        };

        let reserve = if max_quantity > is.quantity() {
            0
        } else {
            is.quantity() - max_quantity
        };

        let move_quantity = is.quantity().min(max_quantity);

        let left_over = self.insert_raw_at(to, is.item_id(), is.max_stack_size(), move_quantity) + reserve;

        self.mut_itemstack_at(from)
            .expect("Already exists because of above if")
            .set_quantity(left_over);

        if left_over == 0 {
            self.set_itemstack_at(from, None);
        }

        Ok(left_over)
    }

    /// Moves an item from slot `from` to slot `to`.
    ///
    /// This will respect stack sizes, and returns the "left over" amount in the slot it was moved from.
    pub fn move_itemstack(
        &mut self,
        from: usize,
        to_inventory: &mut Inventory,
        to: usize,
        max_quantity: u16,
    ) -> Result<u16, InventorySlotError> {
        if from >= self.items.len() {
            return Err(InventorySlotError::InvalidSlot(from));
        }
        if to >= to_inventory.items.len() {
            return Err(InventorySlotError::InvalidSlot(to));
        }

        let is = self.itemstack_at(from);
        let Some(is) = is else {
            return Ok(0);
        };

        let reserve = if max_quantity > is.quantity() {
            0
        } else {
            is.quantity() - max_quantity
        };

        let move_quantity = is.quantity().min(max_quantity);

        let left_over = to_inventory.insert_raw_at(to, is.item_id(), is.max_stack_size(), move_quantity) + reserve;

        self.mut_itemstack_at(from)
            .expect("Already exists because of above if")
            .set_quantity(left_over);

        if left_over == 0 {
            self.set_itemstack_at(from, None);
        }

        Ok(left_over)
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

    /// Checks if the inventory can have this quantity of this item removed from its contents
    pub fn can_take_item(&self, item: &Item, quantity: usize) -> bool {
        self.quantity_of(item) >= quantity
    }

    /// Removes up to the amount specified of this item from the inventory.
    ///
    /// Returns amount that couldn't be taken.
    pub fn take_item(&mut self, item: &Item, mut quantity: usize) -> usize {
        for maybe_is in self
            .items
            .iter_mut()
            .filter(|x| x.as_ref().map(|x| x.item_id() == item.id()).unwrap_or(false))
        {
            let Some(is) = maybe_is else {
                continue;
            };

            let qty = is.quantity();
            if quantity >= qty as usize {
                quantity -= is.quantity() as usize;
                *maybe_is = None;
            } else {
                is.set_quantity(qty - quantity as u16);
                quantity = 0;
            }
        }

        quantity
    }

    /// Iterates over every slot in the inventory.
    pub fn iter(&self) -> std::slice::Iter<'_, Option<ItemStack>> {
        self.items.iter()
    }
}

pub(super) fn register(app: &mut App) {
    itemstack::register(app);
    held_item_slot::register(app);

    app.register_type::<Inventory>().register_type::<HeldItemStack>();
}
