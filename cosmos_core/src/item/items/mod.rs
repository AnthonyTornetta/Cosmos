//! Loads all the items for cosmos & adds the item registry.

use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::prelude::*;

use super::{Item, DEFAULT_MAX_STACK_SIZE};

fn add_cosmos_items(
    mut items: ResMut<Registry<Item>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    items.register(Item::new("cosmos:test_crystal", DEFAULT_MAX_STACK_SIZE));

    items.register(Item::new("cosmos:fluid_cell", DEFAULT_MAX_STACK_SIZE));
    items.register(Item::new("cosmos:fluid_cell_filled", 1));

    items.register(Item::new("cosmos:iron_bar", DEFAULT_MAX_STACK_SIZE));

    loading.finish_loading(id, &mut end_writer);
}

pub(super) fn register<T: States>(app: &mut App, loading_state: T) {
    registry::create_registry::<Item>(app, "cosmos:items");

    app.add_systems(OnEnter(loading_state), add_cosmos_items);
}
