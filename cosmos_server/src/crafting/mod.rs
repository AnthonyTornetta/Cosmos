//! Server-related crafting logic

use bevy::prelude::App;

mod blocks;
mod recipes;

pub(super) fn register(app: &mut App) {
    recipes::register(app);
    blocks::register(app);
}
