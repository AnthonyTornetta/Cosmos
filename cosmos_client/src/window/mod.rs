//! Everything that has to do with the window

use bevy::prelude::App;

mod fullscreen_toggle;
pub mod setup;

pub(super) fn register(app: &mut App) {
    setup::register(app);
    fullscreen_toggle::register(app);
}
