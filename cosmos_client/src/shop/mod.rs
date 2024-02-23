//! Client logic for the shop

use bevy::{
    app::App,
    ecs::{entity::Entity, event::Event},
};
use cosmos_core::{
    shop::{
        netty::{ShopPurchaseError, ShopSellError},
        Shop,
    },
    structure::coordinates::BlockCoordinate,
};

mod netty;
mod ui;

#[derive(Event, Debug)]
/// Sent whenever an item is purchased from the shop.
///
/// The purchase may have been unsuccessful, so make sure to check the details field.
pub struct PurchasedEvent {
    /// The structure that holds the shop
    pub structure_entity: Entity,
    /// The shop's block's coordinates.
    pub shop_block: BlockCoordinate,
    /// If the buying was successful or not.
    pub details: Result<Shop, ShopPurchaseError>,
}

#[derive(Event, Debug)]
/// Sent whenever an item is sold to the shop.
///
/// The selling may have been unsuccessful, so make sure to check the details field.
pub struct SoldEvent {
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

    app.add_event::<PurchasedEvent>().add_event::<SoldEvent>();
}
