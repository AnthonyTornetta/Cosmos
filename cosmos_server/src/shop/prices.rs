//! Temporary: generates default shop prices

use std::fs;

use bevy::{
    app::App,
    ecs::system::Res,
    prelude::{Commands, Resource},
    state::state::OnEnter,
};
use cosmos_core::{
    crafting::recipes::{basic_fabricator::BasicFabricatorRecipes, RecipeItem},
    item::Item,
    registry::Registry,
    state::GameState,
};
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

fn compute_price(base: &[(u16, u32)], item: u16, fab_recipes: &BasicFabricatorRecipes) -> Option<u32> {
    if let Some((_, price)) = base.iter().find(|x| x.0 == item) {
        return Some(*price);
    }

    let recipe = fab_recipes.iter().find(|r| r.output.item == item)?;

    let result = recipe.inputs.iter().map(|i| match i.item {
        RecipeItem::Item(id) => compute_price(base, id, fab_recipes).map(|x| x * i.quantity as u32),
    });

    if result.clone().any(|x| x.is_none()) {
        return None;
    }

    Some(result.flatten().sum())
}

fn create_default_shop_entires(mut commands: Commands, items: Res<Registry<Item>>, fab_recipes: Res<BasicFabricatorRecipes>) {
    let price = |id: &str, price: u32| -> Option<(u16, u32)> { items.from_id(&format!("cosmos:{}", id)).map(|x| (x.id(), price)) };

    let base_prices = [
        price("iron_bar", 10),
        price("copper_bar", 10),
        price("lead_bar", 100),
        price("uranium", 100),
        price("sulfur", 20),
        price("gravitron_crystal", 700),
        price("energite_crystal", 600),
        price("photonium_crystal", 200),
        price("grass", 4),
        price("dirt", 3),
        price("cherry_leaf", 50),
        price("cherry_log", 100),
        price("redwood_log", 30),
        price("redwood_leaf", 20),
        price("molten", 1),
        price("stone", 1),
        price("sand", 3),
        price("cactus", 20),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    /*
        *        COLORS.map(|c| PrettyShopEntry::Selling {
                item_id: format!("cosmos:glass_{c}"),
                max_quantity_selling: 10_000,
                price_per: $price,
            }).flatten(),
    */
    // let entries = fab_recipes.iter().map(|recipe| {
    //     recipe.inputs.iter().map(|x| match x.item {RecipeItem::Item(id) => {
    //         items.from_id()
    //     }})
    // })
    // let mut entries = vec![
    //     p!("iron_bar", 10),
    //     p!("copper_bar", 5),
    //     p!("lead_bar", 100),
    //     p!("uranium", 300),
    //     p!("sulfur", 40),
    //     p!("gravitron_crystal", 1000),
    //     p!("energite_crystal", 600),
    //     p!("photonium_crystal", 300),
    //     p!("uranium_fuel_cell", 3200),
    //     p!("missile", 100),
    //     p!("camera", 50),
    //     p!("gravity_well", 1200),
    //     p!("ramp", 10),
    //     p!("ship_hull_grey", 10),
    //     p!("grass", 20),
    //     p!("stone", 10),
    //     p!("dirt", 10),
    //     p!("cherry_leaf", 50),
    //     p!("cherry_log", 100),
    //     p!("redwood_log", 100),
    //     p!("redwood_leaf", 10),
    //     p!("ship_core", 2000),
    //     p!("energy_cell", 1000),
    //     p!("passive_generator", 1000),
    //     p!("laser_cannon", 1500),
    //     p!("thruster", 400),
    //     p!("light", 100),
    //     p!("glass", 20),
    //     p!("ice", 20),
    //     p!("molten_rock", 10),
    //     p!("sand", 10),
    //     p!("cactus", 30),
    //     p!("build_block", 1200),
    //     p!("reactor_controller", 3000),
    //     p!("reactor_casing", 600),
    //     p!("reactor_window", 600),
    //     p!("reactor_power_cell", 1500),
    //     p!("storage", 50),
    //     p!("station_core", 8000),
    //     p!("plasma_drill", 500),
    // ];
    let entries = items
        .iter()
        .flat_map(|item| compute_price(&base_prices, item.id(), &fab_recipes).map(|p| (item.id(), p)))
        .map(|(item, price)| {
            let item = items.from_numeric_id(item);
            PrettyShopEntry::Selling {
                item_id: item.unlocalized_name().into(),
                max_quantity_selling: 10_000,
                price_per: price,
            }
        })
        .collect::<Vec<_>>();
    // entries.append(
    //     &mut COLORS
    //         .map(|c| PrettyShopEntry::Selling {
    //             item_id: format!("cosmos:glass_{c}"),
    //             max_quantity_selling: 10_000,
    //             price_per: 20,
    //         })
    //         .into_iter()
    //         .collect::<Vec<_>>(),
    // );

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
                    price_per: (price_per as f32 * 0.4) as u32,
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
                item_id: items.from_id(&item_id).unwrap_or_else(|| panic!("Missing {item_id}")).id(),
                max_quantity_buying,
                price_per,
            },
            PrettyShopEntry::Selling {
                item_id,
                max_quantity_selling,
                price_per,
            } => ShopEntry::Selling {
                item_id: items.from_id(&item_id).unwrap_or_else(|| panic!("Missing {item_id}")).id(),
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
