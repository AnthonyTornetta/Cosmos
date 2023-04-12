use bevy::prelude::App;

pub mod asset_loading;

pub fn register(app: &mut App) {
    // shaders::register(app);
    asset_loading::register(app);
}
