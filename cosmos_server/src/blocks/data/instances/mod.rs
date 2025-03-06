use bevy::app::App;

mod basic_fabricator;
mod dye_machine;
mod storage;

pub(super) fn register(app: &mut App) {
    dye_machine::register(app);
    storage::register(app);
    basic_fabricator::register(app);
}
