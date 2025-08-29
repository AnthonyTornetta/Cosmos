use bevy::prelude::*;
use cosmos_core::{
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    netty::sync::IdentifiableComponent,
    registry::Registry,
    state::GameState,
};
use serde::{Deserialize, Serialize};

use crate::{
    items::usable::UseHeldItemEvent,
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
};

#[derive(Component, Serialize, Deserialize, Debug)]
struct BlueprintItemData {
    file_name: String,
}

impl IdentifiableComponent for BlueprintItemData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:blueprint_item_data"
    }
}

impl DefaultPersistentComponent for BlueprintItemData {}

fn on_use_blueprint(
    mut evr_use_item: EventReader<UseHeldItemEvent>,
    q_inventory: Query<&Inventory>,
    q_blueprint_data: Query<&BlueprintItemData>,
) {
}

fn register_blueprint_item(items: Res<Registry<Item>>, mut needs_data: ResMut<ItemShouldHaveData>) {
    if let Some(blueprint_item) = items.from_id("cosmos:blueprint") {
        needs_data.add_item(blueprint_item);
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<BlueprintItemData>(app);

    app.add_systems(OnEnter(GameState::PostLoading), register_blueprint_item);
}
