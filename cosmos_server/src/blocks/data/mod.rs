use bevy::app::App;

mod storage;

pub(super) fn register(app: &mut App) {
    storage::register(app);
}
