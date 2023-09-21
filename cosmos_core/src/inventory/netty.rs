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
}
