//! Represents the communications a laser cannon system sends

use bevy::{ecs::entity::Entity, prelude::Component};
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::BlockCoordinate;

use super::Shop;

#[derive(Debug, Serialize, Deserialize)]
pub enum ShopPurchaseError {
    InsufficientFunds,
    NoStock(Shop),
    NotEnoughInventorySpace,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ShopSellError {
    /// never thrown yet (eventually shops will have their own money)
    InsufficientFunds,
    NotEnoughItems,
    /// never thrown yet (eventually shops will store their items in an inventory)
    NotEnoughInventorySpace,
    NotSellingThatMany(Shop),
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerShopMessages {
    OpenShop {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        shop_data: Shop,
    },
    PurchaseResult {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        details: Result<Shop, ShopPurchaseError>,
    },
    SellResult {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        details: Result<Shop, ShopSellError>,
    },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ClientShopMessages {
    Buy {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        item_id: u16,
        quantity: u32,
    },
    Sell {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        item_id: u16,
        quantity: u32,
    },
}
