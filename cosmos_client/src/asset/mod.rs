use bevy::prelude::App;

pub mod asset_loading;

pub(super) fn register(app: &mut App) {
    asset_loading::register(app);
}
