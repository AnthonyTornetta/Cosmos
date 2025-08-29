//! Client logic for items

use bevy::prelude::App;
use cosmos_core::item::item_category::ItemCategory;

use crate::lang::register_lang;

pub mod descriptions;
pub mod item_mesh;
pub mod physical_item;
mod usable;

pub(super) fn register(app: &mut App) {
    descriptions::register(app);
    item_mesh::register(app);
    physical_item::register(app);
    usable::register(app);

    register_lang::<ItemCategory>(app, vec!["categories"]);
}
