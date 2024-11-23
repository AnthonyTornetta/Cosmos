use bevy::prelude::App;
use serde::{Deserialize, Serialize};

mod basic_fabricator;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RawRecipeItem {
    Item(String),
    Category(String),
}

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
}
