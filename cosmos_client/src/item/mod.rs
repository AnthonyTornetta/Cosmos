//! Client logic for items

use bevy::prelude::App;

pub mod item_mesh;

pub(super) fn register(app: &mut App) {
    item_mesh::register(app);
}
