//! Contains logic for the different types of recipes

use bevy::prelude::App;
use serde::{Deserialize, Serialize};

pub mod basic_fabricator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// An item that is used in a recipe
pub enum RecipeItem {
    /// A single item's numberic id
    Item(u16),
    // Category(u16),
}

pub(super) fn register(app: &mut App) {
    basic_fabricator::register(app);
}
