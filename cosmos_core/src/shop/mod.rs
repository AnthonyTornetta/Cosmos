use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::economy::Credits;

use self::netty::{ShopPurchaseError, ShopSellError};

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

#[derive(Debug, Serialize, Deserialize, Reflect, Default, Component, Clone)]
pub struct Shop {
    pub name: String,
    pub contents: Vec<ShopEntry>,
}

impl Shop {
    pub fn buy(&mut self, item_id: u16, quantity: u32, credits: &mut Credits) -> Result<u32, ShopPurchaseError> {
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

                        return Ok(quantity);
                    }
                }
                _ => {}
            }
        }

        Err(ShopPurchaseError::NoStock(self.clone()))
    }

    pub fn sell(&mut self, item_id: u16, quantity: u32, credits: &mut Credits) -> Result<u32, ShopSellError> {
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
                            return Err(ShopSellError::NotSellingThatMany(self.clone()));
                        }

                        if let Some(max_qty_buying) = max_quantity_buying {
                            *max_qty_buying -= *max_qty_buying - quantity;
                        }

                        credits.increase(credits_gain);

                        return Ok(quantity);
                    }
                }
                _ => {}
            }
        }

        Err(ShopSellError::NotSellingThatMany(self.clone()))
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Shop>().register_type::<ShopEntry>();
}
