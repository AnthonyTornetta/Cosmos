use bevy::prelude::*;
use cosmos_core::inventory::Inventory;

use crate::blocks::{block_events::NoAutoInsertMinedItems, data::utils::add_default_block_data_for_block};

pub(super) fn register(app: &mut App) {
    add_default_block_data_for_block(app, |e, _| Inventory::new("Dye Machine", 1, None, e), "cosmos:dye_machine");
    add_default_block_data_for_block(app, |_, _| NoAutoInsertMinedItems, "cosmos:dye_machine");
}
