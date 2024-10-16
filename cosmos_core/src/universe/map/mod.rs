//! Responsible for map creation and syncing logic

use bevy::prelude::App;

pub mod system;

pub(super) fn register(app: &mut App) {
    system::register(app);
}
