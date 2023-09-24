//! Represents the communications an inventory sends

use bevy::prelude::Entity;
use serde::{Deserialize, Serialize};

use super::Inventory;

#[derive(Debug, Serialize, Deserialize)]
/// All the laser cannon system messages
pub enum ServerInventoryMessages {
    /// Represents the inventory that an entity has.
    EntityInventory {
        /// The serialized version of an inventory.
        inventory: Inventory,
        /// The entity that has this inventory.
        owner: Entity,
    },
}

#[derive(Debug, Serialize, Deserialize)]
/// All the laser cannon system messages
pub enum ClientInventoryMessages {
    /// Asks the server to swap inventory slots.
    SwapSlots {
        /// The first slot
        slot_a: u32,
        /// The entity that has this inventory.
        inventory_a: Entity,
        /// The second slot
        slot_b: u32,
        /// The entity that has this inventory.
        inventory_b: Entity,
    },
    /// Auto moves an item in one inventory to another (or the same)
    AutoMove {
        /// The slot to automove
        from_slot: usize,
        /// The inventory the item is in
        from_inventory: Entity,
        /// The inventory you want to auto-move the item to. Can be the same as `from_inventory` to auto sort it.
        to_inventory: Entity,
    },
}
