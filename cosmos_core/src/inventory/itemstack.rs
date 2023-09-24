//! An ItemStack represents an item & the quantity of that item.

use bevy::{prelude::App, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::{item::Item, registry::identifiable::Identifiable};

#[derive(Serialize, Deserialize, Debug, Reflect, Clone, PartialEq, Eq)]
/// An item & the quantity of that item
pub struct ItemStack {
    item_id: u16,
    quantity: u16,
    max_stack_size: u16,
}

impl ItemStack {
    /// Creates an ItemStack of that item with an initial quantity of 0.
    pub fn new(item: &Item) -> Self {
        Self {
            item_id: item.id(),
            max_stack_size: item.max_stack_size(),
            quantity: 0,
        }
    }

    /// Creates an ItemStack of that item with the given initial quantity
    pub fn with_quantity(item: &Item, quantity: u16) -> Self {
        Self {
            item_id: item.id(),
            max_stack_size: item.max_stack_size(),
            quantity,
        }
    }

    /// Creates an ItemStack of that item id, its max stack size, and with the given initial quantity
    pub fn raw_with_quantity(item_id: u16, max_stack_size: u16, quantity: u16) -> Self {
        Self {
            item_id,
            max_stack_size,
            quantity,
        }
    }

    #[inline]
    /// Gets the item's id
    pub fn item_id(&self) -> u16 {
        self.item_id
    }

    #[inline]
    /// Gets the quantity
    pub fn quantity(&self) -> u16 {
        self.quantity
    }

    #[inline]
    /// Gets the max stack size
    pub fn max_stack_size(&self) -> u16 {
        self.max_stack_size
    }

    #[inline]
    /// Checks if the quantity is 0
    pub fn is_empty(&self) -> bool {
        self.quantity() == 0
    }

    /// Returns the overflow quantity
    pub fn decrease_quantity(&mut self, amount: u16) -> u16 {
        if amount > self.quantity {
            let overflow = amount - self.quantity;

            self.quantity = 0;

            overflow
        } else {
            self.quantity -= amount;

            0
        }
    }

    /// Returns the overflow quantity
    pub fn increase_quantity(&mut self, amount: u16) -> u16 {
        self.quantity += amount;

        if self.quantity > self.max_stack_size {
            let overflow = self.quantity - self.max_stack_size;

            self.quantity = self.max_stack_size;

            overflow
        } else {
            0
        }
    }

    #[inline]
    /// Returns true if the ItemStack is at or above the max stack size.
    pub fn is_full(&self) -> bool {
        self.quantity >= self.max_stack_size
    }

    /// Sets the quantity. Does not care about the max stack size
    pub fn set_quantity(&mut self, new_quantity: u16) {
        self.quantity = new_quantity;
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<ItemStack>();
}
