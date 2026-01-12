use bevy::prelude::*;
use cosmos_core::{item::item_category::ItemCategory, registry::Registry, state::GameState};

fn create_item_categories(mut reg: ResMut<Registry<ItemCategory>>) {
    reg.register(ItemCategory::new("cosmos:build_blocks", "cosmos:ship_hull_dark_grey"));
    reg.register(ItemCategory::new("cosmos:weapons", "cosmos:missile_launcher"));
    reg.register(ItemCategory::new("cosmos:utility", "cosmos:ship_core"));
    reg.register(ItemCategory::new("cosmos:logic", "cosmos:and_gate"));
    reg.register(ItemCategory::new("cosmos:material", "cosmos:iron_bar"));
    reg.register(ItemCategory::new("cosmos:power", "cosmos:energy_cell"));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PreLoading), create_item_categories);
}
