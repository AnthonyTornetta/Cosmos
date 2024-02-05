//! A collection of generic UI elements that can be used

use bevy::app::App;

pub mod text_input;

pub(super) fn register(app: &mut App) {
    text_input::register(app);
}
