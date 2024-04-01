//! Biome-specific generation

use bevy::prelude::App;

pub mod biome;
pub mod block_layers;
pub mod terrain_generation;

pub(super) fn register(app: &mut App) {
    biome::register(app);
}
