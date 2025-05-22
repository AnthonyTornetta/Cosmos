//! Contains logic for blocks that are used for crafting

use bevy::prelude::App;

pub mod advanced_fabricator;
pub mod basic_fabricator;

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
    advanced_fabricator::register(app);
}
