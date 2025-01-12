//! Temporary: generates default shop prices

use std::fs;

use bevy::{
    app::App,
    ecs::system::Res,
    prelude::{Commands, Resource},
    state::state::OnEnter,
};
use cosmos_core::{item::Item, registry::Registry, state::GameState};
use cosmos_core::{registry::identifiable::Identifiable, shop::ShopEntry};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
enum PrettyShopEntry {
    /// The shop is selling this
    Selling {
        /// The item's id
        item_id: String,
        /// The maximum amount the shop is selling
        max_quantity_selling: u32,
        /// The price per item
        price_per: u32,
    },
    /// This shop is buying this
    Buying {
        /// The item's id
        item_id: String,
        /// The maximum amount of this item the shop is buying
        max_quantity_buying: Option<u32>,
        /// The price this shop is willing to pay per item
        price_per: u32,
    },
}

fn create_default_shop_entires(mut commands: Commands, items: Res<Registry<Item>>) {
    let entries = vec![
        PrettyShopEntry::Selling {
            item_id: "cosmos:grass".into(),
            max_quantity_selling: 10_000,
            price_per: 30,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:stone".into(),
            max_quantity_selling: 10_000,
            price_per: 10,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:dirt".into(),
            max_quantity_selling: 10_000,
            price_per: 10,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:laser_cannon".into(),
            max_quantity_selling: 10_000,
            price_per: 300,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:cherry_leaf".into(),
            max_quantity_selling: 10_000,
            price_per: 20,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:redwood_log".into(),
            max_quantity_selling: 10_000,
            price_per: 30,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:redwood_leaf".into(),
            max_quantity_selling: 10_000,
            price_per: 20,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_core".into(),
            max_quantity_selling: 10_000,
            price_per: 1000,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:energy_cell".into(),
            max_quantity_selling: 10_000,
            price_per: 300,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:passive_generator".into(),
            max_quantity_selling: 10_000,
            price_per: 300,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:thruster".into(),
            max_quantity_selling: 10_000,
            price_per: 200,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:light".into(),
            max_quantity_selling: 10_000,
            price_per: 50,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:molten_stone".into(),
            max_quantity_selling: 10_000,
            price_per: 10,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:lava".into(),
            max_quantity_selling: 10_000,
            price_per: 10,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ice".into(),
            max_quantity_selling: 10_000,
            price_per: 30,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:water".into(),
            max_quantity_selling: 10_000,
            price_per: 30,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:sand".into(),
            max_quantity_selling: 10_000,
            price_per: 30,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:cactus".into(),
            max_quantity_selling: 10_000,
            price_per: 50,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:build_block".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_grey".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_black".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_grey".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_white".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_blue".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_blue".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_brown".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_green".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_green".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_orange".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_orange".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_pink".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_pink".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_purple".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_purple".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_red".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_red".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_yellow".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_dark_yellow".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:ship_hull_mint".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_white".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_blue".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_blue".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_brown".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_green".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_green".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_orange".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_orange".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_pink".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_pink".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_purple".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_purple".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_red".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_red".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_yellow".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_dark_yellow".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:glass_mint".into(),
            max_quantity_selling: 10_000,
            price_per: 40,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:reactor_controller".into(),
            max_quantity_selling: 10_000,
            price_per: 1000,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:reactor_casing".into(),
            max_quantity_selling: 10_000,
            price_per: 100,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:reactor_window".into(),
            max_quantity_selling: 10_000,
            price_per: 100,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:reactor_cell".into(),
            max_quantity_selling: 10_000,
            price_per: 200,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:fan".into(),
            max_quantity_selling: 10_000,
            price_per: 10,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:storage".into(),
            max_quantity_selling: 10_000,
            price_per: 100,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:station_core".into(),
            max_quantity_selling: 10_000,
            price_per: 25_000,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:test_ore".into(),
            max_quantity_selling: 10_000,
            price_per: 200,
        },
        PrettyShopEntry::Selling {
            item_id: "cosmos:plasma_drill".into(),
            max_quantity_selling: 10_000,
            price_per: 200,
        },
    ];

    let new_entries = entries
        .into_iter()
        .flat_map(|x| {
            let PrettyShopEntry::Selling {
                item_id,
                max_quantity_selling,
                price_per,
            } = x
            else {
                unreachable!();
            };

            [
                PrettyShopEntry::Selling {
                    item_id: item_id.clone(),
                    max_quantity_selling,
                    price_per,
                },
                PrettyShopEntry::Buying {
                    item_id,
                    max_quantity_buying: None,
                    price_per: (price_per as f32 * 0.9) as u32,
                },
            ]
        })
        .collect::<Vec<PrettyShopEntry>>();

    let json = serde_json::to_string_pretty(&new_entries).unwrap();

    fs::write("./config/cosmos/default_shop.json", json).expect("Couldnt write config file to ./config/cosmos/default_shop.json");

    commands.insert_resource(DefaultShopEntries(get_entries(new_entries, &items)));
}

fn load_default_shop_data(mut commands: Commands, items: Res<Registry<Item>>) {
    let json_data = std::fs::read_to_string("./config/cosmos/default_shop.json").expect("Unable to read config file!");

    let data: Vec<PrettyShopEntry> = serde_json::from_str(&json_data).expect("Bad JSON data!");

    commands.insert_resource(DefaultShopEntries(get_entries(data, &items)));
}

fn get_entries(entries: Vec<PrettyShopEntry>, items: &Registry<Item>) -> Vec<ShopEntry> {
    entries
        .into_iter()
        .map(|x| match x {
            PrettyShopEntry::Buying {
                item_id,
                max_quantity_buying,
                price_per,
            } => ShopEntry::Buying {
                item_id: items.from_id(&item_id).expect("Missing {item_id}").id(),
                max_quantity_buying,
                price_per,
            },
            PrettyShopEntry::Selling {
                item_id,
                max_quantity_selling,
                price_per,
            } => ShopEntry::Selling {
                item_id: items.from_id(&item_id).expect("Missing {item_id}").id(),
                max_quantity_selling,
                price_per,
            },
        })
        .collect::<Vec<ShopEntry>>()
}

#[derive(Resource)]
/// Contains the default entries for a shop
pub struct DefaultShopEntries(pub Vec<ShopEntry>);

pub(super) fn register(app: &mut App) {
    if !std::fs::exists("./config/cosmos/default_shop.json").unwrap_or(false) {
        app.add_systems(OnEnter(GameState::Playing), create_default_shop_entires);
    } else {
        app.add_systems(OnEnter(GameState::Playing), load_default_shop_data);
    }
}
