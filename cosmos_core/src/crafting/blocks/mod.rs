use bevy::prelude::App;

pub mod basic_fabricator;

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
}
