//! Loads all the items for cosmos & adds the item registry.

use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::prelude::*;

use super::{Item, ItemBuilder};

fn add_cosmos_items(
    mut items: ResMut<Registry<Item>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    items.register(
        ItemBuilder::new("cosmos:photonium_crystal")
            .with_category("cosmos:material")
            .create(),
    );

    items.register(ItemBuilder::new("cosmos:fluid_cell").create());
    items.register(ItemBuilder::new("cosmos:fluid_cell_filled").with_stack_size(1).create());

    items.register(ItemBuilder::new("cosmos:iron_bar").with_category("cosmos:material").create());

    items.register(ItemBuilder::new("cosmos:copper_bar").with_category("cosmos:material").create());
    items.register(ItemBuilder::new("cosmos:lead_bar").with_category("cosmos:material").create());
    items.register(ItemBuilder::new("cosmos:uranium").with_category("cosmos:material").create());
    items.register(ItemBuilder::new("cosmos:sulfur").with_category("cosmos:material").create());
    items.register(
        ItemBuilder::new("cosmos:gravitron_crystal")
            .with_category("cosmos:material")
            .create(),
    );
    items.register(
        ItemBuilder::new("cosmos:energite_crystal")
            .with_category("cosmos:material")
            .create(),
    );

    items.register(
        ItemBuilder::new("cosmos:uranium_fuel_cell")
            .with_category("cosmos:material")
            .create(),
    );
    items.register(ItemBuilder::new("cosmos:missile").with_category("cosmos:material").create());

    items.register(ItemBuilder::new("cosmos:magnite").with_category("cosmos:material").create());

    loading.finish_loading(id, &mut end_writer);
}

pub(super) fn register<T: States>(app: &mut App, loading_state: T) {
    registry::create_registry::<Item>(app, "cosmos:items");

    app.add_systems(OnEnter(loading_state), add_cosmos_items);
}
