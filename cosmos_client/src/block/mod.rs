//! This just handles lighting for now.

use bevy::prelude::App;

pub mod lighting;
mod multiblocks;

pub(super) fn register(app: &mut App) {
    multiblocks::register(app);
    lighting::register(app);
}
