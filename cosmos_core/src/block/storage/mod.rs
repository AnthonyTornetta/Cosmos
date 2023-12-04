use bevy::app::App;

pub mod storage_blocks;

pub(super) fn register(app: &mut App) {
    storage_blocks::register(app);
}
