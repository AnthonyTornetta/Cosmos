use bevy::prelude::App;

mod advanced_weapons_fabricator;
mod basic_fabricator;

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
    advanced_weapons_fabricator::register(app);
}
