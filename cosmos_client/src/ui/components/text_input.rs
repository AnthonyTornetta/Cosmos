//! A collection of generic UI elements that can be used

use std::ops::Range;

use arboard::Clipboard;
use bevy::{
    a11y::Focus,
    app::{App, Update},
    color::Color,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{Added, Changed, Or},
        schedule::{common_conditions::resource_changed, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res, ResMut, Resource},
    },
    hierarchy::BuildChildren,
    input::{
        keyboard::{Key, KeyCode, KeyboardInput},
        mouse::MouseButton,
        ButtonInput, ButtonState,
    },
    log::{info, warn},
    prelude::{ChildBuild, Deref, Text, TextUiWriter},
    text::{LineBreak, TextColor, TextFont, TextLayout, TextSpan},
    time::Time,
    ui::{AlignSelf, FocusPolicy, Interaction, Node},
};

use crate::ui::UiSystemSet;

#[derive(Resource, Default)]
struct CursorFlashTime(f32);

#[derive(Deref, Component, Debug, Default)]
/// Holds the value input by the user in this text field.
///
/// This is guarenteed at all times to only contain values that meet the criteria
/// specified in [`InputType`].  **NOTE** For Integer and Decimal, a single negative sign (-)
/// could be stored as the value, in which case parsing will fail but it is considered a valid
/// input.
pub struct InputValue(String);

impl InputValue {
    /// Creates an input value with this value. This is not checked for validity.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Sets the value
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.0 = value.into();
    }

    /// Gets the value.
    ///
    /// ## Warning
    /// If you parse this, ensure you handle the error case properly.
    /// For example, empty strings are valid for `InputType::Decimal` but
    /// will parse badly.
    pub fn value(&self) -> &str {
        &self.0
    }
}

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
#[require(InputValue, Node, TextFont, TextColor)]
/// A text box the user can type in
pub struct TextInput {
    /// Handles input validation to ensure the data stored in [`InputValue`] is correct.
    pub input_type: InputType,
    /// Where the cursor is in the input field.
    pub cursor_pos: usize,
    /// Where the highlighting should begin (an index). This can be before or after the cursor.
    pub highlight_begin: Option<usize>,
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
        }
    }
}

fn monitor_clicked(
    mut focused: ResMut<Focus>,
    mut focused_time: ResMut<CursorFlashTime>,
    mut q_clicked_text_inputs: Query<(Entity, &mut TextInput, &Interaction)>,
    mouse_inputs: Res<ButtonInput<MouseButton>>,
) {
    if mouse_inputs.just_pressed(MouseButton::Left) {
        focused_time.0 = 0.0;

        for (ent, mut text_input, interaction) in q_clicked_text_inputs.iter_mut() {
            text_input.highlight_begin = None;

            if *interaction == Interaction::Pressed {
                focused.0 = Some(ent);
                // } else if focused.0.map(|x| x == ent).unwrap_or(false) {
                // Only clear out focused ent if it was one of the ones we're worried about
                // focused.0 = None;
            }
        }
    }
}

#[derive(Component)]
struct TextEnt(Entity);

fn added_text_input_bundle(
    mut commands: Commands,
    q_added: Query<(Entity, &InputValue, &TextFont, &TextColor), Added<TextInput>>,
    focused: Res<Focus>,
) {
    for (entity, input_value, t_font, t_col) in q_added.iter() {
        commands.entity(entity).insert(Interaction::None).insert(FocusPolicy::Block);

        let mut text_ent = None;

        let mut cursor_style = t_col.clone();
        if focused.0 != Some(entity) {
            cursor_style.0 = Color::NONE;
        }

        commands.entity(entity).with_children(|p| {
            text_ent = Some(
                p.spawn((
                    Name::new("Text input text display"),
                    Text::new(input_value.0.clone()),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..Default::default()
                    },
                    t_font.clone(),
                    t_col.clone(),
                    Node {
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((TextSpan::new("|"), t_font.clone(), cursor_style));
                    p.spawn((TextSpan::new(""), t_font.clone(), t_col.clone()));
                })
                .id(),
            );
        });

        commands.entity(entity).insert(TextEnt(text_ent.expect("Set above")));
    }
}

fn send_key_inputs(
    mut evr_keyboard: EventReader<KeyboardInput>,
    focused: Res<Focus>,
    mut q_focused_input_field: Query<(&mut TextInput, &mut InputValue, &Interaction)>,
    inputs: Res<ButtonInput<KeyCode>>,
) {
    let Some(focused) = focused.0 else {
        // Consumes the event so they don't all pile up and are released when we regain focus
        evr_keyboard.clear();
        return;
    };

    let Ok((mut focused_input_field, mut text, _)) = q_focused_input_field.get_mut(focused) else {
        // Consumes the event so they don't all pile up and are released when we regain focus
        evr_keyboard.clear();
        return;
    };

    for pressed in evr_keyboard.read() {
        if pressed.state != ButtonState::Pressed {
            continue;
        }

        if !inputs.pressed(KeyCode::ControlLeft) && !inputs.pressed(KeyCode::ControlRight) {
            let smol_str = match &pressed.logical_key {
                Key::Character(smol_str) => Some(String::from(smol_str.clone())),
                Key::Space => Some(" ".to_owned()),
                Key::Tab => Some("\t".to_owned()),
                _ => None,
            };

            if let Some(smol_str) = smol_str {
                let mut new_value = text.0.clone();
                let new_cursor_pos;

                if let Some(range) = focused_input_field.get_highlighted_range() {
                    let replace_string = smol_str;
                    new_cursor_pos = range.start + replace_string.len();

                    text.0.replace_range(range, &replace_string);
                } else if focused_input_field.cursor_pos == text.len() {
                    new_value.push_str(smol_str.as_str());

                    new_cursor_pos = focused_input_field.cursor_pos + 1;
                } else {
                    new_value.insert_str(focused_input_field.cursor_pos, smol_str.as_str());

                    new_cursor_pos = focused_input_field.cursor_pos + 1;
                }

                if verify_input(&focused_input_field, &new_value) {
                    text.0 = new_value;
                    focused_input_field.cursor_pos = new_cursor_pos;
                    focused_input_field.highlight_begin = None;
                }
            }
        }

        match pressed.key_code {
            KeyCode::Backspace => {
                if text.is_empty() {
                    continue;
                }

                if let Some(range) = focused_input_field.get_highlighted_range() {
                    focused_input_field.cursor_pos = range.start;
                    focused_input_field.highlight_begin = None;

                    text.0.replace_range(range, "");
                } else if focused_input_field.cursor_pos != 0 {
                    text.0.remove(focused_input_field.cursor_pos - 1);
                    focused_input_field.cursor_pos -= 1;
                }
            }
            KeyCode::Delete => {
                if text.is_empty() {
                    continue;
                }

                if let Some(range) = focused_input_field.get_highlighted_range() {
                    focused_input_field.cursor_pos = range.start;
                    focused_input_field.highlight_begin = None;

                    text.0.replace_range(range, "");
                } else if focused_input_field.cursor_pos != text.len() {
                    text.0.remove(focused_input_field.cursor_pos);
                }
            }
            _ => {}
        }
    }
}

fn show_text_cursor(mut writer: TextUiWriter, focused: Res<Focus>, q_text_inputs: Query<(Entity, &TextEnt)>) {
    for (ent, text) in q_text_inputs.iter() {
        if focused.0.map(|x| x == ent).unwrap_or(false) {
            let col = writer.color(text.0, 0).0.clone();
            writer.color(text.0, 1).0 = col;
        } else {
            writer.color(text.0, 1).0 = Color::NONE;
        }
    }
}

fn flash_cursor(
    mut cursor_flash_time: ResMut<CursorFlashTime>,
    focused: Res<Focus>,
    q_text_inputs: Query<&TextEnt>,
    time: Res<Time>,
    mut writer: TextUiWriter,
) {
    const CURSOR_FLASH_SPEED: f32 = 1.0; // # flashes per second

    let Some(focused_ent) = focused.0 else {
        return;
    };

    let Ok(text) = q_text_inputs.get(focused_ent) else {
        return;
    };

    let col = writer.color(text.0, 0).0.clone();

    let empty = writer.text(text.0, 1).is_empty();
    let mut c = writer.color(text.0, 1);
    if (cursor_flash_time.0 * CURSOR_FLASH_SPEED * 2.0) as i64 % 2 == 0 {
        if c.as_ref().0 != col {
            c.as_mut().0 = col;
        }
    } else if !empty && c.as_ref().0 != Color::NONE {
        c.as_mut().0 = Color::NONE;
    }

    cursor_flash_time.0 += time.delta_secs();
}

fn value_changed(
    focused: Res<Focus>,
    mut q_values_changed: Query<(Entity, &InputValue, &mut TextInput, &TextEnt), Or<(Changed<InputValue>, Changed<TextInput>)>>,
    mut cursor_flash_time: ResMut<CursorFlashTime>,
    mut writer: TextUiWriter,
) {
    for (entity, input_val, mut text_input, text) in q_values_changed.iter_mut() {
        if focused.0.map(|x| x == entity).unwrap_or(false) {
            cursor_flash_time.0 = 0.0;
        }

        // If something modified the InputValue externally, the cursor pos may be outside the number, so make sure it isn't
        if text_input.cursor_pos > input_val.len() {
            text_input.cursor_pos = input_val.len();
        }

        input_val[0..text_input.cursor_pos].clone_into(writer.text(text.0, 0).as_mut());
        if text_input.cursor_pos < input_val.len() {
            input_val[text_input.cursor_pos..input_val.len()].clone_into(writer.text(text.0, 2).as_mut());
        } else {
            *writer.text(text.0, 2).as_mut() = "".into();
        }
    }
}

fn handle_keyboard_shortcuts(
    focused: Res<Focus>,
    mut q_text_inputs: Query<(&mut InputValue, &mut TextInput)>,
    mut cursor_flash_time: ResMut<CursorFlashTime>,
    inputs: Res<ButtonInput<KeyCode>>,
    mut evr_keyboard: EventReader<KeyboardInput>,
) {
    let Some(focused_entity) = focused.0 else {
        evr_keyboard.clear();
        return;
    };

    let Ok((mut value, mut text_input)) = q_text_inputs.get_mut(focused_entity) else {
        evr_keyboard.clear();
        return;
    };

    for pressed in evr_keyboard.read() {
        if pressed.state != ButtonState::Pressed {
            continue;
        }

        if inputs.pressed(KeyCode::ControlLeft) || inputs.pressed(KeyCode::ControlRight) {
            match pressed.key_code {
                KeyCode::KeyA => {
                    if !value.is_empty() {
                        text_input.highlight_begin = Some(0);
                        text_input.cursor_pos = value.len();
                    }

                    cursor_flash_time.0 = 0.0;
                }
                KeyCode::KeyC => {
                    let Ok(mut clipboard) = Clipboard::new() else {
                        continue;
                    };

                    let Some(range) = text_input.get_highlighted_range() else {
                        continue;
                    };

                    if let Err(err) = clipboard.set_text(&value[range]) {
                        warn!("{err}");
                    }
                }
                KeyCode::KeyX => {
                    let Ok(mut clipboard) = Clipboard::new() else {
                        continue;
                    };

                    let Some(range) = text_input.get_highlighted_range() else {
                        continue;
                    };

                    if let Err(err) = clipboard.set_text(&value[range.clone()]) {
                        warn!("{err}");
                    }

                    let mut new_value = value.0.clone();
                    let new_cursor_pos = range.start;
                    new_value.replace_range(range, "");

                    if verify_input(&text_input, &new_value) {
                        value.0 = new_value;
                        text_input.cursor_pos = new_cursor_pos;
                        text_input.highlight_begin = None;
                    }
                }
                KeyCode::KeyV => {
                    let Ok(mut clipboard) = Clipboard::new() else {
                        continue;
                    };

                    let Ok(clipboard_contents) = clipboard.get_text() else {
                        continue;
                    };

                    let mut new_value = value.0.clone();
                    let new_cursor_pos;

                    if let Some(range) = text_input.get_highlighted_range() {
                        let replace_string = clipboard_contents;
                        new_cursor_pos = range.start + replace_string.len();

                        new_value.replace_range(range, &replace_string);
                    } else if text_input.cursor_pos == value.len() {
                        new_value.push_str(&clipboard_contents);

                        new_cursor_pos = clipboard_contents.len() + text_input.cursor_pos;
                    } else {
                        new_value.insert_str(text_input.cursor_pos, &clipboard_contents);

                        new_cursor_pos = clipboard_contents.len() + text_input.cursor_pos;
                    }

                    if verify_input(&text_input, &new_value) {
                        value.0 = new_value;
                        text_input.cursor_pos = new_cursor_pos;
                        text_input.highlight_begin = None;
                    }
                }
                KeyCode::ArrowLeft => {
                    if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
                        text_input.highlight_begin = Some(text_input.cursor_pos);
                    } else {
                        text_input.highlight_begin = None;
                    }
                    text_input.cursor_pos = 0;
                    cursor_flash_time.0 = 0.0;
                }
                KeyCode::ArrowRight => {
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
            match pressed.key_code {
                KeyCode::ArrowLeft => {
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
                KeyCode::ArrowRight => {
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
            info!("Highlighted: {slice}");
        }
    }
}

fn verify_input(text_input: &TextInput, test_value: &str) -> bool {
    match text_input.input_type {
        InputType::Text { max_length } => max_length.map(|max_len| test_value.len() <= max_len).unwrap_or(true),
        InputType::Integer { min, max } => test_value == "-" || test_value.parse::<i64>().map(|x| x >= min && x <= max).unwrap_or(false),
        InputType::Decimal { min, max } => test_value == "-" || test_value.parse::<f64>().map(|x| x >= min && x <= max).unwrap_or(false),
        InputType::Custom(check) => check(test_value),
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the TextInput component uses. Make sure you add any [`TextInput`] components before this set!
pub enum TextInputUiSystemSet {
    /// Make sure you add any [`TextInput`] components before this set!
    ///
    /// Sets up any [`TextInput`] components added.
    AddTextInputBundle,
    /// Updates the slider to contain any values that have been set via the "react" system
    HandleReactValues,
    /// Sends user input to the various [`TextInput`] components.
    SendKeyInputs,
    /// Updates any components based on the value being changed in this [`TextInput`]
    ///
    /// The results of this can be read in [`InputValue`].
    ValueChanged,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            TextInputUiSystemSet::AddTextInputBundle,
            TextInputUiSystemSet::HandleReactValues,
            TextInputUiSystemSet::SendKeyInputs,
            TextInputUiSystemSet::ValueChanged,
        )
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            added_text_input_bundle.in_set(TextInputUiSystemSet::AddTextInputBundle),
            (
                monitor_clicked,
                show_text_cursor.run_if(resource_changed::<Focus>),
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
