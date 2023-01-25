// use crate::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};
use crate::registry::{self};
use bevy::ecs::schedule::StateData;
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

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    pre_loading_state: T,
    _loading_state: T,
) {
    registry::register::<T, Item>(app, pre_loading_state);

    // app.add_system_set(SystemSet::on_enter(loading_state).with_system(add_cosmos_items));
}
