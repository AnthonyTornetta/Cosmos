use bevy::app::App;

mod advanced_weapons_fabricator;
mod basic_fabricator;
mod dye_machine;
mod railgun;
mod storage;

pub(super) fn register(app: &mut App) {
    advanced_weapons_fabricator::register(app);
    dye_machine::register(app);
    storage::register(app);
    railgun::register(app);
    basic_fabricator::register(app);
}
