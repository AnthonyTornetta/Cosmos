//! Loads all the items for cosmos & adds the item registry.

use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::prelude::*;

use super::{DEFAULT_MAX_STACK_SIZE, Item};

fn add_cosmos_items(
    mut items: ResMut<Registry<Item>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    items.register(Item::new("cosmos:photonium_crystal", DEFAULT_MAX_STACK_SIZE));

    items.register(Item::new("cosmos:fluid_cell", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:fluid_cell_filled", 1));

    items.register(Item::new("cosmos:iron_bar", DEFAULT_MAX_STACK_SIZE));

    items.register(Item::new("cosmos:copper_bar", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:lead_bar", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:uranium", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:sulfur", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:gravitron_crystal", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:energite_crystal", DEFAULT_MAX_STACK_SIZE));

    items.register(Item::new("cosmos:uranium_fuel_cell", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:missile", DEFAULT_MAX_STACK_SIZE));

    items.register(Item::new("cosmos:magnite", DEFAULT_MAX_STACK_SIZE));

    loading.finish_loading(id, &mut end_writer);
}

pub(super) fn register<T: States>(app: &mut App, loading_state: T) {
    registry::create_registry::<Item>(app, "cosmos:items");

    app.add_systems(OnEnter(loading_state), add_cosmos_items);
}
