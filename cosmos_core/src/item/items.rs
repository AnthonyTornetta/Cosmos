use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self, Registry};
use bevy::ecs::schedule::StateData;
use bevy::prelude::{App, EventWriter, ResMut, SystemSet};

use super::{Item, DEFAULT_MAX_STACK_SIZE};

pub fn add_cosmos_items(
    mut items: ResMut<Registry<Item>>,
    mut loading: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loading.register_loader(&mut start_writer);

    let item_ids = [
        "stone",
        "grass",
        "dirt",
        "cherry_leaf",
        "cherry_log",
        "ship_core",
        "energy_cell",
        "laser_cannon",
    ];

    for id in item_ids {
        let cid = format!("cosmos:{}", id);
        items.register(Item::new(cid, DEFAULT_MAX_STACK_SIZE));
    }

    loading.finish_loading(id, &mut end_writer);
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    loading_state: T,
) {
    registry::register::<T, Item>(app, pre_loading_state);

    app.add_system_set(SystemSet::on_enter(loading_state).with_system(add_cosmos_items));
}
