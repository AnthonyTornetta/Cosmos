//! Temporary: generates default shop prices

use bevy::{
    app::App,
    ecs::{schedule::OnEnter, system::Res},
    prelude::{Commands, Resource},
};
use cosmos_core::registry::identifiable::Identifiable;
use cosmos_core::shop::ShopEntry;
use cosmos_core::{item::Item, registry::Registry};

use crate::state::GameState;

fn create_default_shop_entires(mut commands: Commands, items: Res<Registry<Item>>) {
    let entries = vec![
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:grass").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 30,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:stone").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 10,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:dirt").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 10,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:laser_cannon").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 300,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:cherry_leaf").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 20,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:redwood_log").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 30,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:redwood_leaf").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 20,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_core").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 1000,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:energy_cell").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 300,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:reactor").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 300,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:thruster").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 200,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:light").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 50,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 50,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:molten_stone").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 10,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:cheese").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 10,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ice").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 30,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:water").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 30,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:sand").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 30,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:cactus").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 50,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:build_block").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_grey").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_black").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_grey").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_white").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_blue").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_blue").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_brown").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_green").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_green").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_orange").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_orange").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_pink").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_pink").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_purple").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_purple").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_red").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_red").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_yellow").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_dark_yellow").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:ship_hull_mint").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_white").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_blue").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_blue").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_brown").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_green").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_green").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_orange").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_orange").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_pink").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_pink").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_purple").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_purple").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_red").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_red").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_yellow").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_dark_yellow").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:glass_mint").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 40,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:reactor_controller").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 1000,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:reactor_casing").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 100,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:reactor_window").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 100,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:reactor_cell").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 200,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:fan").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 10,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:storage").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 100,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:station_core").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 25_000,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:test_ore").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 200,
        },
        ShopEntry::Selling {
            item_id: items.from_id("cosmos:plasma_drill").expect("Missing Item").id(),
            max_quantity_selling: 100000,
            price_per: 200,
        },
    ];

    let new_entries = entries
        .into_iter()
        .flat_map(|x| {
            let ShopEntry::Selling {
                item_id,
                max_quantity_selling: _,
                price_per,
            } = x
            else {
                panic!(":O")
            };

            [
                x,
                ShopEntry::Buying {
                    item_id,
                    max_quantity_buying: None,
                    price_per: (price_per as f32 * 0.9) as u32,
                },
            ]
        })
        .collect::<Vec<ShopEntry>>();

    println!("{}", serde_json::to_string_pretty(&new_entries).unwrap());

    commands.insert_resource(DefaultShopEntries(new_entries));
}

fn load_default_shop_data(mut commands: Commands) {
    let json_data = std::fs::read_to_string("./config/cosmos/default_shop.json").expect("Unable to read config file!");

    let data: Vec<ShopEntry> = serde_json::from_str(&json_data).expect("Bad JSON data!");

    commands.insert_resource(DefaultShopEntries(data));
}

#[derive(Resource)]
/// Contains the default entries for a shop
pub struct DefaultShopEntries(pub Vec<ShopEntry>);

pub(super) fn register(app: &mut App) {
    if !std::fs::try_exists("./config/cosmos/default_shop.json").unwrap_or(false) {
        app.add_systems(OnEnter(GameState::Playing), create_default_shop_entires);
    } else {
        app.add_systems(OnEnter(GameState::Playing), load_default_shop_data);
    }
}
