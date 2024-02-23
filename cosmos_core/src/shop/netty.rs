//! Represents the communications a laser cannon system sends

use bevy::{ecs::entity::Entity, prelude::Component};
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::BlockCoordinate;

use super::Shop;

#[derive(Debug, Serialize, Deserialize)]
/// An error occurred when trying to buy something from the shop
pub enum ShopPurchaseError {
    /// The buyer lacked sufficient credits to fulfill the purchase
    InsufficientFunds,
    /// The shop didn't have enough stock for this purchase
    NoStock(Shop),
    /// The buyer didn't have enough room in their inventory to fit the items
    NotEnoughInventorySpace,
}

#[derive(Debug, Serialize, Deserialize)]
/// An error occurred when trying to sell something to the shop
pub enum ShopSellError {
    /// never thrown yet (eventually shops will have their own money)
    InsufficientFunds,
    /// The buyer did not have enough items to sell
    NotEnoughItems,
    /// never thrown yet (eventually shops will store their items in an inventory)
    NotEnoughInventorySpace,
    /// The shop isn't willing to buy that many items
    NotWillingToBuyThatMany(Shop),
}

#[derive(Debug, Serialize, Deserialize, Component)]
/// Messages about shops the server will send to the player
pub enum ServerShopMessages {
    /// Tells the client to open a shop menu
    OpenShop {
        /// The shop to open's block
        shop_block: BlockCoordinate,
        /// The shop to open's structure entity
        structure_entity: Entity,
        /// The data about the shop
        shop_data: Shop,
    },
    /// Sent whenever an attempt to purchase something from the shop is handled
    PurchaseResult {
        /// The shop's block
        shop_block: BlockCoordinate,
        /// The shop's entity
        structure_entity: Entity,
        /// The details about the purchase
        details: Result<Shop, ShopPurchaseError>,
    },
    /// Sent whenever an attempt to sell something to the shop is handled
    SellResult {
        /// The shop's block
        shop_block: BlockCoordinate,
        /// The shop's entity
        structure_entity: Entity,
        /// The details about the selling
        details: Result<Shop, ShopSellError>,
    },
}

#[derive(Debug, Serialize, Deserialize, Component)]
/// Sent from the client to the server to communicate about shop items.
pub enum ClientShopMessages {
    /// Client qequests to buy something from the shop
    Buy {
        /// The shop they're buying from's block coordinates
        shop_block: BlockCoordinate,
        /// The shop they're buying from's structure entity
        structure_entity: Entity,
        /// The item they are buying
        item_id: u16,
        /// The quantity they want to buy
        quantity: u32,
    },
    /// Client wants to sell something to the shop
    Sell {
        /// The shop they're selling to's block coordinates
        shop_block: BlockCoordinate,
        /// The shop they're selling to's structure entity
        structure_entity: Entity,
        /// The item they are selling
        item_id: u16,
        /// The quantity they want to sell
        quantity: u32,
    },
}
