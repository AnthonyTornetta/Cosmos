//! Everything that has to do with the window

use bevy::prelude::App;

pub mod setup;

pub(super) fn register(app: &mut App) {
    setup::register(app);
}
