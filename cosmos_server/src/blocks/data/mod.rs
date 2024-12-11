use bevy::app::App;

mod basic_fabricator;
mod storage;

pub(super) fn register(app: &mut App) {
    storage::register(app);
    basic_fabricator::register(app);
}
