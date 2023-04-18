//! Loads all the items for cosmos & adds the item registry.

// use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry;
use bevy::prelude::App;

use super::Item;

// pub fn add_cosmos_items(
//     mut items: ResMut<Registry<Item>>,
//     mut loading: ResMut<LoadingManager>,
//     mut end_writer: EventWriter<DoneLoadingEvent>,
//     mut start_writer: EventWriter<AddLoadingEvent>,
// ) {
//     let id = loading.register_loader(&mut start_writer);
//     loading.finish_loading(id, &mut end_writer);
// }

pub(super) fn register(app: &mut App) {
    registry::create_registry::<Item>(app);

    // app.add_system_set(SystemSet::on_enter(loading_state).with_system(add_cosmos_items));
}
