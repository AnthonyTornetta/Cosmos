use std::fs;

use bevy::{app::App, ecs::system::Commands};

#[derive(Debug, Clone)]
struct Price {
    unlocalized_name: String,
    id: u16,

    price: u64,
}

// fn load_prices(mut prices: Commands) {
//     let Ok(prices) = fs::read("config/cosmos/default_shop.json") else {

//     }
// }

// pub struct DefaultShopPrices()

pub(super) fn register(app: &mut App) {}
