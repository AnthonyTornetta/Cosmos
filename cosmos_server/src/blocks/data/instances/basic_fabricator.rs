//! Handles basic fabricator inventory

use bevy::app::App;
use cosmos_core::inventory::Inventory;

use crate::blocks::data::utils::add_default_block_data_for_block;

pub(super) fn register(app: &mut App) {
    add_default_block_data_for_block(
        app,
        |e, _| Inventory::new("Basic Fabricator", 6, None, e),
        "cosmos:basic_fabricator",
    );
}
