//! Responsible for map creation and syncing logic

use bevy::prelude::App;

pub mod system;
pub mod territory;

pub(super) fn register(app: &mut App) {
    system::register(app);
    territory::register(app);
}
