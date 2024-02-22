//! Temporary: generates default shop prices

use bevy::{
    app::App,
    ecs::{schedule::OnEnter, system::Res},
};
use cosmos_core::{item::Item, registry::Registry};

use crate::state::GameState;

fn create_default_shop_entires(items: Res<Registry<Item>>) {
    let entries = [
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:grass").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 30,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:stone").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 10,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:dirt").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 10,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:log").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 10,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:laser_cannon").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 300,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:cherry_leaf").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 20,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:redwood_log").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 30,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:redwood_leaf").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 20,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_core").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 1000,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:energy_cell").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 300,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:reactor").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 300,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:thruster").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 200,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:light").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 50,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 50,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:molten_stone").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 10,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:cheese").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 10,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ice").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 30,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:water").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 30,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:sand").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 30,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:cactus").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 50,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:build_block").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_grey").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_black").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_grey").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_white").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_blue").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_blue").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_brown").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_green").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_green").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_orange").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_orange").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_pink").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_pink").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_purple").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_purple").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_red").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_red").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_yellow").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_dark_yellow").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:ship_hull_mint").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_white").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_blue").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_blue").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_brown").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_green").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_green").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_orange").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_orange").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_pink").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_pink").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_purple").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_purple").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_red").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_red").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_yellow").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_dark_yellow").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:glass_mint").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 40,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:reactor_controller").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 1000,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:reactor_casing").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 100,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:reactor_window").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 100,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:reactor_cell").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 200,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:fan").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 10,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:storage").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 100,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:station_core").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 25_000,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:test_ore").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 200,
    },
    ShopEntry::Buying {
        item_id: items.from_id("cosmos:plasma_drill").expect("Missing Item")),
        max_quantity_buying: 100000,
        price_per: 200,
    },
    ];
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), create_default_shop_entires);
}
