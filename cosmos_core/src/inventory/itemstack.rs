use bevy::prelude::App;
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use serde::{Deserialize, Serialize};

use crate::{item::Item, registry::identifiable::Identifiable};

#[derive(Serialize, Deserialize, Debug, Inspectable)]
pub struct ItemStack {
    item_id: u16,
    quantity: u16,
    max_stack_size: u16,
}

impl ItemStack {
    pub fn new(item: &Item) -> Self {
        Self {
            item_id: item.id(),
            max_stack_size: item.max_stack_size(),
            quantity: 0,
        }
    }

    pub fn with_quantity(item: &Item, quantity: u16) -> Self {
        Self {
            item_id: item.id(),
            max_stack_size: item.max_stack_size(),
            quantity,
        }
    }

    #[inline]
    pub fn item_id(&self) -> u16 {
        self.item_id
    }

    #[inline]
    pub fn quantity(&self) -> u16 {
        self.quantity
    }

    #[inline]
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
    pub fn is_full(&self) -> bool {
        self.quantity >= self.max_stack_size
    }
}

pub fn register(app: &mut App) {
    app.register_inspectable::<ItemStack>();
}
