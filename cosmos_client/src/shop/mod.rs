//! Client logic for the shop

use bevy::prelude::*;
use cosmos_core::{
    shop::{
        Shop,
        netty::{ShopPurchaseError, ShopSellError},
    },
    structure::coordinates::BlockCoordinate,
};

mod netty;
mod ui;

#[derive(Message, Debug)]
/// Sent whenever an item is purchased from the shop.
///
/// The purchase may have been unsuccessful, so make sure to check the details field.
pub struct PurchasedMessage {
    /// The structure that holds the shop
    pub structure_entity: Entity,
    /// The shop's block's coordinates.
    pub shop_block: BlockCoordinate,
    /// If the buying was successful or not.
    pub details: Result<Shop, ShopPurchaseError>,
}

#[derive(Message, Debug)]
/// Sent whenever an item is sold to the shop.
///
/// The selling may have been unsuccessful, so make sure to check the details field.
pub struct SoldMessage {
    /// The structure that holds the shop
    pub structure_entity: Entity,
    /// The shop's block's coordinates.
    pub shop_block: BlockCoordinate,
    /// If the selling was successful or not.
    pub details: Result<Shop, ShopSellError>,
}

pub(super) fn register(app: &mut App) {
    ui::register(app);
    netty::register(app);

    app.add_message::<PurchasedMessage>().add_message::<SoldMessage>();
}
