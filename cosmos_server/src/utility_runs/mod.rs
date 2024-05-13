//! This folder should be mini programs that are run to facilitate normal maintenance of game files.
//!
//! For example, updating save data to new data format

use bevy::app::App;

pub mod block_flipper;
pub mod structure_updater;

pub(super) fn register(_app: &mut App) {
    // block_flipper::register(app);
    // structure_updater::register(_app);
}
