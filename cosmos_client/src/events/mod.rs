//! This module is stupid and should be removed. The submodules of this should be moved into the actual block/ship modules instead of this.
//!
//! too bad

use bevy::prelude::App;

pub mod block;
pub mod ship;

pub(super) fn register(app: &mut App) {
    block::register(app);
    ship::register(app);
}
