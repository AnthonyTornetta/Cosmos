//! Client logic for items

use bevy::prelude::App;

pub mod item_mesh;
pub mod physical_item;

pub(super) fn register(app: &mut App) {
    item_mesh::register(app);
    physical_item::register(app);
}
