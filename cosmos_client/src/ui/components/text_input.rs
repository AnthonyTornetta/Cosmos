//! A collection of generic UI elements that can be used

use std::ops::Range;

use bevy::{
    a11y::Focus,
    app::{App, Update},
    asset::AssetServer,
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{Added, Changed, Or, With},
        schedule::{apply_deferred, common_conditions::resource_changed, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    input::{
        keyboard::{KeyCode, KeyboardInput},
        mouse::MouseButton,
        ButtonState, Input,
    },
    prelude::Deref,
    render::color::Color,
    text::{Text, TextSection, TextStyle},
    time::Time,
    ui::{node_bundles::TextBundle, FocusPolicy, Interaction},
    window::ReceivedCharacter,
};

#[derive(Resource, Default)]
struct CursorFlashTime(f32);

#[derive(Deref, Component, Debug, Default)]
/// Holds the value input by the user in this text field.
///
/// This is guarenteed at all times to only contain values that meet the criteria
/// specified in [`InputType`].
pub struct InputValue(String);

#[derive(Debug)]
/// Used to validate user input for a given TextInput field.
pub enum InputType {
    /// Input can by anything, with an optional maximum length.
    Text {
        /// The optional maximum length of the user's input
        max_length: Option<usize>,
    },
    /// Input can only be a valid `i64` value.
    Integer {
        /// Minimum value
        min: i64,
        /// Maximum value
        max: i64,
    },
    /// Input can only be a valid `f64` value.
    Decimal {
        /// Minimum value
        min: f64,
        /// Maximum value
        max: f64,
    },
    /// Input only valid if the given callback returns true for that input
    Custom(fn(&str) -> bool),
}

#[derive(Component, Debug)]
/// A text box the user can type in
pub struct TextInput {
    input_type: InputType,
    cursor_pos: usize,
    highlight_begin: Option<usize>,
    /// The style of the text
    pub style: TextStyle,
}

impl TextInput {
    fn get_highlighted_range(&self) -> Option<Range<usize>> {
        self.highlight_begin.map(|highlighted| {
            let start = highlighted.min(self.cursor_pos);
            let end = highlighted.max(self.cursor_pos);

            start..end
        })
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            cursor_pos: 0,
            highlight_begin: None,
            input_type: InputType::Text { max_length: None },
            style: TextStyle {
                color: Color::WHITE,
                font_size: 12.0,
                font: Default::default(), // font assigned later
            },
        }
    }
}

#[derive(Bundle, Debug, Default)]
/// An easy way of creating the [`TextInput`] UI element.
pub struct TextInputBundle {
    /// The text bundle that will be used with the TextInput
    pub text_bundle: TextBundle,
    /// The [`TextInput`] used to capture user input
    pub text_input: TextInput,
    /// The component you can read to get what the user types
    pub value: InputValue,
}

fn monitor_clicked(
    mut focused: ResMut<Focus>,
    mut focused_time: ResMut<CursorFlashTime>,
    mut q_clicked_text_inputs: Query<(Entity, &mut TextInput, &Interaction)>,
    mouse_inputs: Res<Input<MouseButton>>,
) {
    if mouse_inputs.just_pressed(MouseButton::Left) {
        focused_time.0 = 0.0;

        for (ent, mut text_input, interaction) in q_clicked_text_inputs.iter_mut() {
            text_input.highlight_begin = None;

            if *interaction == Interaction::Pressed {
                focused.0 = Some(ent);
            } else if focused.0.map(|x| x == ent).unwrap_or(false) {
                // Only clear out focused ent if it was one of the ones we're worried about
                focused.0 = None;
            }
        }
    }
}

fn added_text_input_bundle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut q_added: Query<(Entity, &mut Text, &InputValue, &mut TextInput), Added<TextInput>>,
) {
    for (entity, mut text, input_value, mut text_input) in q_added.iter_mut() {
        commands.entity(entity).insert(Interaction::None).insert(FocusPolicy::Block);

        if text.sections.len() != 2 {
            if text_input.style.font == Default::default() {
                // Font hasn't been assigned yet if it's the default handle, so assign the default now.
                text_input.style.font = asset_server.load("fonts/PixeloidSans.ttf");
            }

            let text_style = TextStyle {
                color: Color::WHITE,
                font_size: 22.0,
                font: text_input.style.font.clone(),
            };

            text.sections.clear();

            text.sections.push(TextSection {
                style: text_style.clone(),
                value: input_value.0.clone(),
            });
            text.sections.push(TextSection {
                style: text_style.clone(),
                value: "|".into(),
            });
            text.sections.push(TextSection {
                style: text_style,
                value: "".into(),
            });
        }
    }
}

fn send_key_inputs(
    mut evr_keyboard: EventReader<KeyboardInput>,
    mut evr_char: EventReader<ReceivedCharacter>,
    focused: Res<Focus>,
    mut q_focused_input_field: Query<(&mut TextInput, &mut InputValue, &Interaction)>,
) {
    let Some(focused) = focused.0 else {
        // Consumes the event so they don't all pile up and are released when we regain focus
        evr_char.clear();
        evr_keyboard.clear();
        return;
    };

    let Ok((mut focused_input_field, mut text, _)) = q_focused_input_field.get_mut(focused) else {
        // Consumes the event so they don't all pile up and are released when we regain focus
        evr_char.clear();
        evr_keyboard.clear();
        return;
    };

    for pressed in evr_keyboard.read() {
        if pressed.state != ButtonState::Pressed {
            continue;
        }

        match pressed.key_code {
            Some(KeyCode::Back) => {
                if focused_input_field.cursor_pos == 0 {
                    continue;
                }
                if text.is_empty() {
                    continue;
                }

                if let Some(range) = focused_input_field.get_highlighted_range() {
                    focused_input_field.cursor_pos = range.start;
                    focused_input_field.highlight_begin = None;

                    text.0.replace_range(range, "");
                } else {
                    text.0.remove(focused_input_field.cursor_pos - 1);
                    focused_input_field.cursor_pos -= 1;
                }
            }
            Some(KeyCode::Delete) => {
                if focused_input_field.cursor_pos == text.len() {
                    continue;
                }
                if text.is_empty() {
                    continue;
                }

                if let Some(range) = focused_input_field.get_highlighted_range() {
                    focused_input_field.cursor_pos = range.start;
                    focused_input_field.highlight_begin = None;

                    text.0.replace_range(range, "");
                } else {
                    text.0.remove(focused_input_field.cursor_pos);
                }
            }
            _ => {}
        }
    }

    for ev in evr_char.read() {
        if !ev.char.is_control() {
            let mut new_value = text.0.clone();

            if let Some(range) = focused_input_field.get_highlighted_range() {
                let replace_string = ev.char.to_string();
                focused_input_field.cursor_pos = range.start + replace_string.len();
                focused_input_field.highlight_begin = None;

                text.0.replace_range(range, &replace_string);
            } else if focused_input_field.cursor_pos == text.len() {
                new_value.push(ev.char);
            } else {
                new_value.insert(focused_input_field.cursor_pos, ev.char);
            }

            if match focused_input_field.input_type {
                InputType::Text { max_length } => max_length.map(|max_len| new_value.len() <= max_len).unwrap_or(true),
                InputType::Integer { min, max } => new_value.parse::<i64>().map(|x| x >= min && x <= max).unwrap_or(false),
                InputType::Decimal { min, max } => new_value.parse::<f64>().map(|x| x >= min && x <= max).unwrap_or(false),
                InputType::Custom(check) => check(&new_value),
            } {
                text.0 = new_value;
                focused_input_field.cursor_pos += 1;
            }
        }
    }
}

fn show_text_cursor(focused: Res<Focus>, mut q_text_inputs: Query<(Entity, &mut Text, &TextInput)>) {
    for (ent, mut text, text_input) in q_text_inputs.iter_mut() {
        if focused.0.map(|x| x == ent).unwrap_or(false) {
            text.sections[1].style.color = text_input.style.color;
        } else {
            text.sections[1].style.color = Color::NONE;
        }
    }
}

fn flash_cursor(
    mut cursor_flash_time: ResMut<CursorFlashTime>,
    focused: Res<Focus>,
    mut q_text_inputs: Query<&mut Text, With<TextInput>>,
    time: Res<Time>,
) {
    const CURSOR_FLASH_SPEED: f32 = 1.0; // # flashes per second

    let Some(focused_ent) = focused.0 else {
        return;
    };

    let Ok(mut text) = q_text_inputs.get_mut(focused_ent) else {
        return;
    };

    if (cursor_flash_time.0 * CURSOR_FLASH_SPEED * 2.0) as i64 % 2 == 0 {
        if text.sections[1].value.is_empty() {
            text.sections[1].value = "|".into();
        }
    } else if !text.sections[1].value.is_empty() {
        text.sections[1].value.clear();
    }

    cursor_flash_time.0 += time.delta_seconds();
}

fn value_changed(
    focused: Res<Focus>,
    mut q_values_changed: Query<(Entity, &InputValue, &TextInput, &mut Text), Or<(Changed<InputValue>, Changed<TextInput>)>>,
    mut cursor_flash_time: ResMut<CursorFlashTime>,
) {
    for (entity, input_val, text_input, mut text) in q_values_changed.iter_mut() {
        if focused.0.map(|x| x == entity).unwrap_or(false) {
            cursor_flash_time.0 = 0.0;
        }

        text.sections[0].value = input_val[0..text_input.cursor_pos].to_owned();
        if text_input.cursor_pos < input_val.len() {
            text.sections[2].value = input_val[text_input.cursor_pos..input_val.len()].to_owned();
        } else {
            text.sections[2].value = "".into();
        }
    }
}

fn handle_keyboard_shortcuts(
    focused: Res<Focus>,
    mut q_text_inputs: Query<(&InputValue, &mut TextInput)>,
    mut cursor_flash_time: ResMut<CursorFlashTime>,
    inputs: Res<Input<KeyCode>>,
    mut evr_keyboard: EventReader<KeyboardInput>,
) {
    let Some(focused_entity) = focused.0 else {
        evr_keyboard.clear();
        return;
    };

    let Ok((value, mut text_input)) = q_text_inputs.get_mut(focused_entity) else {
        evr_keyboard.clear();
        return;
    };

    for pressed in evr_keyboard.read() {
        if pressed.state != ButtonState::Pressed {
            continue;
        }

        let Some(keycode) = pressed.key_code else {
            continue;
        };

        if inputs.pressed(KeyCode::ControlLeft) || inputs.pressed(KeyCode::ControlRight) {
            match keycode {
                KeyCode::A => {
                    if !value.is_empty() {
                        text_input.highlight_begin = Some(0);
                        text_input.cursor_pos = value.len();
                    }

                    cursor_flash_time.0 = 0.0;
                }
                KeyCode::Left => {
                    println!("Left!");

                    if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
                        text_input.highlight_begin = Some(text_input.cursor_pos);
                    } else {
                        text_input.highlight_begin = None;
                    }
                    text_input.cursor_pos = 0;
                    cursor_flash_time.0 = 0.0;
                }
                KeyCode::Right => {
                    println!("Right!");
                    if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
                        text_input.highlight_begin = Some(text_input.cursor_pos);
                    } else {
                        text_input.highlight_begin = None;
                    }
                    text_input.cursor_pos = value.len();
                    cursor_flash_time.0 = 0.0;
                }
                _ => {}
            }
        } else {
            match keycode {
                KeyCode::Left => {
                    if text_input.cursor_pos != 0 {
                        if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
                            if text_input.highlight_begin.is_none() {
                                text_input.highlight_begin = Some(text_input.cursor_pos);
                            }
                        } else {
                            text_input.highlight_begin = None;
                        }
                        text_input.cursor_pos -= 1;
                    }
                    cursor_flash_time.0 = 0.0;
                }
                KeyCode::Right => {
                    if text_input.cursor_pos != value.len() {
                        if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
                            if text_input.highlight_begin.is_none() {
                                text_input.highlight_begin = Some(text_input.cursor_pos);
                            }
                        } else {
                            text_input.highlight_begin = None;
                        }
                        text_input.cursor_pos += 1;
                    }
                    cursor_flash_time.0 = 0.0;
                }
                _ => {}
            }
        }
    }

    if let Some(highlighted) = text_input.highlight_begin {
        if highlighted == text_input.cursor_pos {
            text_input.highlight_begin = None;
        } else {
            let start = highlighted.min(text_input.cursor_pos);
            let end = highlighted.max(text_input.cursor_pos);

            let slice = &value[start..end];
            println!("Highlighted: {slice}");
        }
    }
}

// https://github.com/bevyengine/bevy/pull/9822
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the TextInput component uses. Make sure you add any [`TextInput`] components before this set!
pub enum TextInputUiSystemSet {
    /// apply_deferred
    ApplyDeferredA,
    /// Make sure you add any [`TextInput`] components before this set!
    ///
    /// Sets up any [`TextInput`] components added.
    AddTextInputBundle,
    /// apply_deferred
    ApplyDeferredB,
    /// Sends user input to the various [`TextInput`] components.
    SendKeyInputs,
    /// apply_deferred
    ApplyDeferredC,
    /// Updates any components based on the value being changed in this [`TextInput`]
    ///
    /// The results of this can be read in [`InputValue`].
    ValueChanged,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            TextInputUiSystemSet::ApplyDeferredA,
            TextInputUiSystemSet::AddTextInputBundle,
            TextInputUiSystemSet::ApplyDeferredB,
            TextInputUiSystemSet::SendKeyInputs,
            TextInputUiSystemSet::ApplyDeferredC,
            TextInputUiSystemSet::ValueChanged,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            apply_deferred.in_set(TextInputUiSystemSet::ApplyDeferredA),
            apply_deferred.in_set(TextInputUiSystemSet::ApplyDeferredB),
            apply_deferred.in_set(TextInputUiSystemSet::ApplyDeferredC),
        ),
    )
    .add_systems(
        Update,
        (
            added_text_input_bundle.in_set(TextInputUiSystemSet::AddTextInputBundle),
            (
                monitor_clicked,
                show_text_cursor.run_if(resource_changed::<Focus>()),
                handle_keyboard_shortcuts,
                flash_cursor,
                send_key_inputs,
            )
                .chain()
                .in_set(TextInputUiSystemSet::SendKeyInputs),
            value_changed.in_set(TextInputUiSystemSet::ValueChanged),
        ),
    )
    .init_resource::<CursorFlashTime>();
}
