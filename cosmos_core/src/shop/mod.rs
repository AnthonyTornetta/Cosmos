use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

pub mod netty;

#[derive(Debug, Serialize, Deserialize, Reflect, Component, Clone, Copy, PartialEq, Eq)]
pub enum ShopEntry {
    Selling {
        item_id: u16,
        max_quantity_selling: u32,
        price_per: u32,
    },
    Buying {
        item_id: u16,
        max_quantity_buying: Option<u32>,
        price_per: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, Reflect, Default)]
pub struct Shop {
    pub name: String,
    pub contents: Vec<ShopEntry>,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Shop>().register_type::<ShopEntry>();
}
