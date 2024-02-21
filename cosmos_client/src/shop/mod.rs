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
pub struct PurchasedEvent {
    pub structure_entity: Entity,
    pub shop_block: BlockCoordinate,
    pub details: Result<Shop, ShopPurchaseError>,
}

#[derive(Event, Debug)]
pub struct SoldEvent {
    pub structure_entity: Entity,
    pub shop_block: BlockCoordinate,
    pub details: Result<Shop, ShopSellError>,
}

pub(super) fn register(app: &mut App) {
    ui::register(app);
    netty::register(app);

    app.add_event::<PurchasedEvent>().add_event::<SoldEvent>();
}
