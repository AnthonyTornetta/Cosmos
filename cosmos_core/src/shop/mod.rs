//! Facilitate the trading of goods

use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::economy::Credits;

use self::netty::{ShopPurchaseError, ShopSellError};

pub mod netty;

#[derive(Debug, Serialize, Deserialize, Reflect, Component, Clone, Copy, PartialEq, Eq)]
/// The entries a shop can have
pub enum ShopEntry {
    /// The shop is selling this
    Selling {
        /// The item's id
        item_id: u16,
        /// The maximum amount the shop is selling
        max_quantity_selling: u32,
        /// The price per item
        price_per: u32,
    },
    /// This shop is buying this
    Buying {
        /// The item's id
        item_id: u16,
        /// The maximum amount of this item the shop is buying
        max_quantity_buying: Option<u32>,
        /// The price this shop is willing to pay per item
        price_per: u32,
    },
}

#[derive(Debug, Serialize, Deserialize, Reflect, Default, Component, Clone)]
/// Block data that indiciates this is a shop
pub struct Shop {
    /// The name of the shop
    pub name: String,
    /// What the shop is buying/selling
    pub contents: Vec<ShopEntry>,
}

impl Shop {
    /// Buys an item from this shop, or returns an error if the purchase was unsuccessful
    pub fn buy(&mut self, item_id: u16, quantity: u32, credits: &mut Credits) -> Result<(), ShopPurchaseError> {
        for entry in self.contents.iter_mut() {
            match entry {
                ShopEntry::Selling {
                    item_id: entry_id,
                    max_quantity_selling,
                    price_per,
                } => {
                    if *entry_id == item_id {
                        let cost = *price_per as u64 * quantity as u64;

                        if *max_quantity_selling < quantity {
                            return Err(ShopPurchaseError::NoStock(self.clone()));
                        }

                        if !credits.decrease(cost) {
                            return Err(ShopPurchaseError::InsufficientFunds);
                        }

                        *max_quantity_selling -= quantity;

                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        Err(ShopPurchaseError::NoStock(self.clone()))
    }

    /// Sells an item to this shop, or returns an error if the selling was unsuccessful
    pub fn sell(&mut self, item_id: u16, quantity: u32, credits: &mut Credits) -> Result<(), ShopSellError> {
        for entry in self.contents.iter_mut() {
            match entry {
                ShopEntry::Buying {
                    item_id: entry_id,
                    max_quantity_buying,
                    price_per,
                } => {
                    if *entry_id == item_id {
                        let credits_gain = *price_per as u64 * quantity as u64;

                        if max_quantity_buying.unwrap_or(u32::MAX) < quantity {
                            return Err(ShopSellError::NotWillingToBuyThatMany(self.clone()));
                        }

                        if let Some(max_qty_buying) = max_quantity_buying {
                            *max_qty_buying -= *max_qty_buying - quantity;
                        }

                        credits.increase(credits_gain);

                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        Err(ShopSellError::NotWillingToBuyThatMany(self.clone()))
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Shop>().register_type::<ShopEntry>();
}
