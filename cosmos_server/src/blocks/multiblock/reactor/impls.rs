use bevy::prelude::App;
use cosmos_core::{block::multiblock::reactor::Reactors, inventory::Inventory};

use crate::{
    blocks::data::utils::add_default_block_data_for_block,
    persistence::make_persistent::{make_persistent, DefaultPersistentComponent},
};

impl DefaultPersistentComponent for Reactors {}

pub(super) fn register(app: &mut App) {
    add_default_block_data_for_block(app, |e, _| Inventory::new("Reactor", 1, None, e), "cosmos:reactor_controller");
    make_persistent::<Reactors>(app);
}
