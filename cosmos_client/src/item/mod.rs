//! Client logic for items

use bevy::prelude::App;

pub mod descriptions;
pub mod item_mesh;
pub mod physical_item;

pub(super) fn register(app: &mut App) {
    descriptions::register(app);
    item_mesh::register(app);
    physical_item::register(app);
}
