use bevy::prelude::App;

pub mod asset_loading;
pub mod shaders;

pub fn register(app: &mut App) {
    // shaders::register(app);
    asset_loading::register(app);
}
