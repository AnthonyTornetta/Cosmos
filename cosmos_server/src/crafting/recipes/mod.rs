use bevy::prelude::App;
use serde::{Deserialize, Serialize};

mod advanced_fabricator;
mod basic_fabricator;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RawRecipeItem {
    Item(String),
    // Category(String),
}

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
    advanced_fabricator::register(app);
}
