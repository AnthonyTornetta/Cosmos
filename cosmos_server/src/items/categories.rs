use bevy::prelude::*;
use cosmos_core::{
    item::{Item, item_category::ItemCategory},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

fn create_item_categories(mut reg: ResMut<Registry<ItemCategory>>) {
    reg.register(ItemCategory::new("cosmos:hull", "cosmos:ship_hull_dark_grey"));
    reg.register(ItemCategory::new("cosmos:weapons", "cosmos:missile_launcher"));
    reg.register(ItemCategory::new("cosmos:utility", "cosmos:ship_core"));
    reg.register(ItemCategory::new("cosmos:logic", "cosmos:and_gate"));
    reg.register(ItemCategory::new("cosmos:materials", "cosmos:iron_bar"));
}

fn add_item_categories(mut categories: ResMut<Registry<ItemCategory>>, items: Res<Registry<Item>>) {
    println!("[");
    for item in items.iter() {
        println!("(\"{}\", ),", item.unlocalized_name());
    }
    println!("]");
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PreLoading), create_item_categories);

    app.add_systems(OnEnter(GameState::Playing), add_item_categories);
}
