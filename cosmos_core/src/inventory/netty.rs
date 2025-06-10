//! Represents the communications an inventory sends

use bevy::prelude::Entity;
use serde::{Deserialize, Serialize};

use crate::block::data::BlockDataIdentifier;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
/// A way of identifying where the inventory is
pub enum InventoryIdentifier {
    /// The inventory is attached to this entity
    Entity(Entity),
    /// The inventory is for thie specific block data
    BlockData(BlockDataIdentifier),
}

#[derive(Debug, Serialize, Deserialize)]
/// All the inventory messages
pub enum ServerInventoryMessages {
    /// Called whenever a player tries to open an inventory that isn't their own
    OpenInventory {
        /// The owner of the inventory
        owner: InventoryIdentifier,
    },
}

#[derive(Debug, Serialize, Deserialize)]
/// All the client inventory messages
pub enum ClientInventoryMessages {
    /// Asks the server to swap inventory slots.
    SwapSlots {
        /// The first slot
        slot_a: u32,
        /// The entity that has this inventory.
        inventory_a: InventoryIdentifier,
        /// The second slot
        slot_b: u32,
        /// The entity that has this inventory.
        inventory_b: InventoryIdentifier,
    },
    /// Auto moves an item in one inventory to another (or the same)
    AutoMove {
        /// The slot to automove
        from_slot: u32,
        /// The amount to move
        quantity: u16,
        /// The inventory the item is in
        from_inventory: InventoryIdentifier,
        /// The inventory you want to auto-move the item to. Can be the same as `from_inventory` to auto sort it.
        to_inventory: InventoryIdentifier,
    },
    /// Picks up the itemstack at this slot and makes that the held itemstack
    ///
    /// Note that this can only be used when you are not already holding an itemstack, and will do nothing if you are
    PickupItemstack {
        /// The inventory's entity
        inventory_holder: InventoryIdentifier,
        /// The slot to pickup from
        slot: u32,
        /// The amount of the held item to pick up from the inventory (is checked on the server to not exceed the held quantity)
        ///
        /// Feel free to use `u16::MAX` to pick up as many items as possible
        quantity: u16,
    },
    /// Inserts a specified quantity of the itemstack into this slot
    DepositHeldItemstack {
        /// The inventory's entity
        inventory_holder: InventoryIdentifier,
        /// The slot you are inserting into
        slot: u32,
        /// The amount of the held item to insert into the inventory (is checked on the server to not exceed the held quantity)
        ///
        /// Feel free to use `u16::MAX` to insert as many items as possible
        quantity: u16,
    },
    /// Deposits the held itemstack into any available slot in this player's inventory, otherwise
    /// drops it.
    DropOrDepositHeldItemstack,
    /// Deposits all the items in the itemstack into that slot, and makes the item that is currently in this slot the held item
    DepositAndSwapHeldItemstack {
        /// The entity that has this inventory you're interacting with
        inventory_holder: InventoryIdentifier,
        /// The slot you want to swap the held item with
        slot: u32,
    },
    /// Manually moves an itemstack in one inventory to another (or the same) inventory.
    MoveItemstack {
        /// The slot to automove
        from_slot: u32,
        /// The maximum amount to move
        quantity: u16,
        /// The inventory the item is in
        from_inventory: InventoryIdentifier,
        /// The inventory you want to auto-move the item to. Can be the same as `from_inventory` to auto sort it.
        to_inventory: InventoryIdentifier,
        /// The slot to go to
        to_slot: u32,
    },
    /// "Throws" the currently held item in the cursor
    ///
    /// Note throwing isn't implemented yet, so for now it will simply delete the item.
    ThrowHeldItemstack {
        /// The amount of the held item to throw (is checked on the server to not exceed the held quantity)
        quantity: u16,
    },
    /// "Throws" the currently held item in the cursor
    ///
    /// Note throwing isn't implemented yet, so for now it will simply delete the item.
    ThrowItemstack {
        /// The entity that has this inventory
        inventory_holder: InventoryIdentifier,
        /// The amount of the held item to throw (is checked on the server to not exceed the held quantity)
        quantity: u16,
        /// The slot of the inventory you are throwing
        slot: u32,
    },
    /// "Throws" the currently held item in the cursor
    ///
    /// Note throwing isn't implemented yet, so for now it will simply delete the item.
    InsertHeldItem {
        /// The amount of the held item to insert into the inventory (is checked on the server to not exceed the held quantity)
        quantity: u16,
        /// The entity that has this inventory attached to it you want to insert into
        inventory_holder: InventoryIdentifier,
    },
}
