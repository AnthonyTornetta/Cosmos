use bevy::prelude::App;
use serde::{Deserialize, Serialize};

pub mod basic_fabricator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecipeItem {
    Item(u16),
    Category(u16),
}

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
}
