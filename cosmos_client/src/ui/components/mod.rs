//! A collection of generic UI elements that can be used

use bevy::{app::App, ecs::component::Component};

pub mod button;
pub mod scollable_container;
pub mod show_cursor;
pub mod slider;
pub mod text_input;
pub mod window;

#[derive(Component)]
/// If this is on an item with user input, user input will be ignored
pub struct Disabled;

pub(super) fn register(app: &mut App) {
    text_input::register(app);
    button::register(app);
    slider::register(app);
    scollable_container::register(app);
    window::register(app);
    show_cursor::register(app);
}
