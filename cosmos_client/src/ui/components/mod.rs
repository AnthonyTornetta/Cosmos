//! A collection of generic UI elements that can be used

use bevy::{input_focus::InputFocus, prelude::*};

use super::UiSystemSet;

pub mod button;
pub mod scollable_container;
pub mod show_cursor;
pub mod slider;
pub mod tabbed_view;
pub mod text_input;
pub mod window;

#[derive(Component)]
/// If this is on an item with user input, user input will be ignored
pub struct Disabled;

fn clear_focus(
    mut focused: ResMut<InputFocus>,
    q_interaction: Query<(Entity, &Interaction), Without<Disabled>>,
    mouse_inputs: Res<ButtonInput<MouseButton>>,
) {
    if mouse_inputs.just_pressed(MouseButton::Left) {
        if let Some((ent, _)) = q_interaction
            .iter()
            .find(|(_, interaction)| !matches!(interaction, Interaction::None))
        {
            focused.0 = Some(ent);
        } else {
            focused.0 = None;
        }
    }
}

pub(super) fn register(app: &mut App) {
    text_input::register(app);
    button::register(app);
    slider::register(app);
    scollable_container::register(app);
    window::register(app);
    show_cursor::register(app);
    tabbed_view::register(app);

    app.add_systems(Update, clear_focus.in_set(UiSystemSet::PreDoUi));
}
