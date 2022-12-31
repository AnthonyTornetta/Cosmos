use bevy::prelude::{App, Component};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use serde::{Deserialize, Serialize};

use crate::{item::Item, registry::identifiable::Identifiable};

use self::itemstack::ItemStack;

pub mod itemstack;

// TODO
// pub enum InventoryType {
//     BulkInventory,   // These inventories are not organizable by the player
//     NormalInventory, // These inventories are organizable by the player
// }

#[derive(Default, Component, Serialize, Deserialize, Debug, Inspectable)]
pub struct Inventory {
    items: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn new(n_slots: usize) -> Self {
        let mut items = Vec::with_capacity(n_slots);

        for _ in 0..n_slots {
            items.push(None);
        }

        Self { items }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns the overflow that could not fit
    pub fn insert(&mut self, item: &Item, mut quantity: u16) -> u16 {
        // Search for existing stacks, if none found that make new one(s)

        for space in &mut self.items {
            if let Some(is) = space {
                if is.item_id() == item.id() {
                    quantity = is.increase_quantity(quantity);

                    if quantity == 0 {
                        break;
                    }
                }
            }
        }

        // no suitable locations found with pre-existing stacks of that item, make new ones

        for i in 0..self.items.len() {
            if self.items[i].is_none() {
                let mut is = ItemStack::new(item);
                quantity = is.increase_quantity(quantity);

                self.items[i] = Some(is);

                if quantity == 0 {
                    break;
                }
            }
        }

        // if any amount is left over, it will be represented in the quantity variable

        quantity
    }

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

    /// Swaps the ItemStack's places in the inventory
    pub fn swap(&mut self, slot_a: usize, slot_b: usize) {
        self.items.swap(slot_a, slot_b);
    }

    pub fn set_itemstack_at(&mut self, slot: usize, item_stack: Option<ItemStack>) {
        self.items[slot] = item_stack;
    }

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

    pub fn quantity_of(&self, item: &Item) -> usize {
        self.items
            .iter()
            .filter(|x| x.is_some())
            .map(|x| x.as_ref().unwrap())
            .filter(|x| x.item_id() == item.id())
            .map(|x| x.quantity() as usize)
            .sum()
    }
}

pub fn register(app: &mut App) {
    itemstack::register(app);
    app.register_inspectable::<Inventory>();
}
