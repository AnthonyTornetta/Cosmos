//! A collection of generic UI elements that can be used

use bevy::picking::hover::PickingInteraction;
use bevy::{input_focus::InputFocus, prelude::*};

use super::UiSystemSet;

pub mod button;
pub mod checkbox;
pub mod focus;
pub mod modal;
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
    q_interaction: Query<(Entity, &PickingInteraction, Has<bevy::ui_widgets::TextInput>), Without<Disabled>>,
    mouse_inputs: Res<ButtonInput<MouseButton>>,
) {
    if mouse_inputs.just_pressed(MouseButton::Left) {
        if let Some((ent, _, _)) = q_interaction
            .iter()
            .filter(|(_, interaction, _)| !matches!(interaction, PickingInteraction::None))
            .max_by_key(|(_, _, is_text_input)| *is_text_input)
        {
            focused.set(ent, bevy::input_focus::FocusCause::Pressed);
        } else {
            focused.clear();
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
    modal::register(app);
    focus::register(app);
    checkbox::register(app);

    app.add_systems(Update, clear_focus.in_set(UiSystemSet::PreDoUi))
        .init_resource::<InputFocus>();
}
