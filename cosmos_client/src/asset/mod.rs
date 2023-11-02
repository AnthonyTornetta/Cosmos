//! Contains systems & resources for loading & using assets

use bevy::prelude::App;

pub mod asset_loader;
pub mod asset_loading;
pub mod block_materials;
pub mod repeating_material;

pub(super) fn register(app: &mut App) {
    asset_loading::register(app);
    repeating_material::register(app);
    block_materials::register(app);
}
