//! A collection of generic UI elements that can be used

use std::ops::Range;

use arboard::Clipboard;
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    input_focus::InputFocus,
    prelude::*,
    text::LineHeight,
    ui::FocusPolicy,
};
use bevy_ui_text_input::{
    TextInputBuffer, TextInputFilter, TextInputMode, TextInputNode, TextInputPrompt, TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
};
use cosmic_text::{Buffer, BufferLine, Cursor, Edit, Selection};

use crate::ui::UiSystemSet;

use super::show_cursor::any_open_menus;

#[derive(Resource, Default)]
struct CursorFlashTime(f32);

#[derive(Deref, Component, Debug, Default, Reflect)]
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

#[derive(Debug, Reflect, Clone, Copy)]
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
    // /// Input only valid if the given callback returns true for that input
    // Custom(fn(&str) -> bool),
}

impl Default for InputType {
    fn default() -> Self {
        Self::Text { max_length: None }
    }
}

#[derive(Component, Debug, Reflect)]
#[require(InputValue, Node, TextFont, TextColor)]
/// A text box the user can type in
pub struct TextInput {
    /// Handles input validation to ensure the data stored in [`InputValue`] is correct.
    pub input_type: InputType,
    /// Where the cursor is in the input field.
    pub cursor_pos: usize,
    /// The node that the text will be placed onto
    pub text_node: Node,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            cursor_pos: 0,
            input_type: InputType::Text { max_length: None },
            text_node: Node {
                align_self: AlignSelf::Center,
                ..Default::default()
            },
        }
    }
}

#[derive(Component)]
struct TextEnt(Entity);

fn create_text_input_queue(initial_text: &str) -> TextInputQueue {
    let mut queue = TextInputQueue::default();
    let overwrite_mode = false;
    for char in initial_text.chars() {
        queue.add(TextInputAction::Edit(TextInputEdit::Insert(char, overwrite_mode)));
    }
    queue
}

fn added_text_input_bundle(
    mut commands: Commands,
    mut q_added: Query<
        (
            Entity,
            &mut Node,
            &ComputedNode,
            Option<&TextLayout>,
            &InputValue,
            &TextFont,
            &TextColor,
            &TextInput,
        ),
        Added<TextInput>,
    >,
) {
    for (entity, mut node, computed_node, text_layout, input_value, t_font, t_col, ti) in q_added.iter_mut() {
        if node.height == Val::Auto {
            // Auto doesn't work correctly
            node.height = Val::Px(40.0);
        }

        commands.entity(entity).insert(Interaction::None).insert(FocusPolicy::Block);

        let mut text_ent = None;

        let height = computed_node.size().y;

        commands.entity(entity).with_children(|p| {
            let max_chars = if let InputType::Text { max_length } = &ti.input_type {
                max_length.clone()
            } else {
                None
            };

            let mut ecmds = p.spawn((
                TextFont {
                    line_height: LineHeight::Px(height),
                    ..t_font.clone()
                },
                TextInputNode {
                    mode: TextInputMode::SingleLine,
                    max_chars,
                    ..Default::default()
                },
                create_text_input_queue(input_value.value()),
                *t_col,
                TextInputPrompt {
                    text: "World Name".into(),
                    ..Default::default()
                },
                Node {
                    width: Val::Percent(100.0),
                    ..Default::default()
                },
                Pickable::default(),
                Name::new("Text input"),
            ));

            if let Some(text_layout) = text_layout {
                ecmds.insert(text_layout.clone());
            }

            match ti.input_type {
                InputType::Integer { min, max } => {
                    ecmds.insert(TextInputFilter::custom(move |s| {
                        let Ok(num) = s.parse::<i64>() else {
                            return false;
                        };

                        num >= min && num <= max
                    }));
                }
                InputType::Decimal { min, max } => {
                    ecmds.insert(TextInputFilter::custom(move |s| {
                        let Ok(num) = s.parse::<f64>() else {
                            return false;
                        };

                        num >= min && num <= max
                    }));
                }
                InputType::Text { max_length: _ } => {
                    // Handled above
                }
            }

            text_ent = Some(ecmds.id());
        });

        commands.entity(entity).insert(TextEnt(text_ent.expect("Set above")));
    }
}

// fn send_key_inputs(
//     mut evr_keyboard: MessageReader<KeyboardInput>,
//     focused: Res<InputFocus>,
//     mut q_focused_input_field: Query<(&mut TextInput, &mut InputValue, &Interaction)>,
//     inputs: Res<ButtonInput<KeyCode>>,
// ) {
//     let Some(focused) = focused.0 else {
//         // Consumes the event so they don't all pile up and are released when we regain focus
//         evr_keyboard.clear();
//         return;
//     };
//
//     let Ok((mut focused_input_field, mut text, _)) = q_focused_input_field.get_mut(focused) else {
//         // Consumes the event so they don't all pile up and are released when we regain focus
//         evr_keyboard.clear();
//         return;
//     };
//
//     for pressed in evr_keyboard.read() {
//         if pressed.state != ButtonState::Pressed {
//             continue;
//         }
//
//         if !inputs.pressed(KeyCode::ControlLeft) && !inputs.pressed(KeyCode::ControlRight) {
//             let smol_str = match &pressed.logical_key {
//                 Key::Character(smol_str) => Some(String::from(smol_str.clone())),
//                 Key::Space => Some(" ".to_owned()),
//                 Key::Tab => Some("\t".to_owned()),
//                 _ => None,
//             };
//
//             if let Some(smol_str) = smol_str {
//                 let mut new_value = text.0.clone();
//                 let new_cursor_pos;
//
//                 if let Some(range) = focused_input_field.get_highlighted_range() {
//                     let replace_string = smol_str;
//                     new_cursor_pos = range.start + replace_string.len();
//
//                     text.0.replace_range(range, &replace_string);
//                 } else if focused_input_field.cursor_pos == text.len() {
//                     new_value.push_str(smol_str.as_str());
//
//                     new_cursor_pos = focused_input_field.cursor_pos + 1;
//                 } else {
//                     new_value.insert_str(focused_input_field.cursor_pos, smol_str.as_str());
//
//                     new_cursor_pos = focused_input_field.cursor_pos + 1;
//                 }
//
//                 if verify_input(&focused_input_field, &new_value) {
//                     text.0 = new_value;
//                     focused_input_field.cursor_pos = new_cursor_pos;
//                     focused_input_field.highlight_begin = None;
//                 }
//             }
//         }
//
//         match pressed.key_code {
//             KeyCode::Backspace => {
//                 if text.is_empty() {
//                     continue;
//                 }
//
//                 if let Some(range) = focused_input_field.get_highlighted_range() {
//                     focused_input_field.cursor_pos = range.start;
//                     focused_input_field.highlight_begin = None;
//
//                     text.0.replace_range(range, "");
//                 } else if focused_input_field.cursor_pos != 0 {
//                     text.0.remove(focused_input_field.cursor_pos - 1);
//                     focused_input_field.cursor_pos -= 1;
//                 }
//             }
//             KeyCode::Delete => {
//                 if text.is_empty() {
//                     continue;
//                 }
//
//                 if let Some(range) = focused_input_field.get_highlighted_range() {
//                     focused_input_field.cursor_pos = range.start;
//                     focused_input_field.highlight_begin = None;
//
//                     text.0.replace_range(range, "");
//                 } else if focused_input_field.cursor_pos != text.len() {
//                     text.0.remove(focused_input_field.cursor_pos);
//                 }
//             }
//             _ => {}
//         }
//     }
// }
//
// fn show_text_cursor(mut writer: TextUiWriter, focused: Res<InputFocus>, q_text_inputs: Query<(Entity, &TextEnt)>) {
//     for (ent, text) in q_text_inputs.iter() {
//         if focused.0.map(|x| x == ent).unwrap_or(false) {
//             let col = writer.color(text.0, 0).0;
//             writer.color(text.0, 1).0 = col;
//         } else {
//             writer.color(text.0, 1).0 = Color::NONE;
//         }
//     }
// }
//
// fn flash_cursor(
//     mut cursor_flash_time: ResMut<CursorFlashTime>,
//     focused: Res<InputFocus>,
//     q_text_inputs: Query<&TextEnt>,
//     time: Res<Time>,
//     mut writer: TextUiWriter,
// ) {
//     const CURSOR_FLASH_SPEED: f32 = 1.0; // # flashes per second
//
//     let Some(focused_ent) = focused.0 else {
//         return;
//     };
//
//     let Ok(text) = q_text_inputs.get(focused_ent) else {
//         return;
//     };
//
//     let col = writer.color(text.0, 0).0;
//
//     let empty = writer.text(text.0, 1).is_empty();
//     let mut c = writer.color(text.0, 1);
//     if (cursor_flash_time.0 * CURSOR_FLASH_SPEED * 2.0) as i64 % 2 == 0 {
//         if c.as_ref().0 != col {
//             c.as_mut().0 = col;
//         }
//     } else if !empty && c.as_ref().0 != Color::NONE {
//         c.as_mut().0 = Color::NONE;
//     }
//
//     cursor_flash_time.0 += time.delta_secs();
// }

fn value_changed(
    focused: Res<InputFocus>,
    mut q_values_changed: Query<(Entity, &InputValue, &mut TextInput, &TextEnt), Or<(Changed<InputValue>, Changed<TextInput>)>>,
    mut cursor_flash_time: ResMut<CursorFlashTime>,
    mut writer: TextUiWriter,
) {
    for (entity, input_val, mut text_input, text) in q_values_changed.iter_mut() {
        let mut input_buffer = TextInputBuffer::default();
        let editor = &mut input_buffer.editor;
        // editor.set_selection(Selection::Line(Cursor {
        //     line: 0,
        //     index: 0,
        //     affinity: Default::default(),
        // }));
        editor.insert_string(input_val.value(), None);

        // if focused.0.map(|x| x == entity).unwrap_or(false) {
        //     cursor_flash_time.0 = 0.0;
        // }
        //
        // // If something modified the InputValue externally, the cursor pos may be outside the number, so make sure it isn't
        // if text_input.cursor_pos > input_val.len() {
        //     text_input.cursor_pos = input_val.len();
        // }
        //
        // input_val[0..text_input.cursor_pos].clone_into(writer.text(text.0, 0).as_mut());
        // if text_input.cursor_pos < input_val.len() {
        //     input_val[text_input.cursor_pos..input_val.len()].clone_into(writer.text(text.0, 2).as_mut());
        // } else {
        //     *writer.text(text.0, 2).as_mut() = "".into();
        // }
    }
}

// fn handle_keyboard_shortcuts(
//     focused: Res<InputFocus>,
//     mut q_text_inputs: Query<(&mut InputValue, &mut TextInput)>,
//     mut cursor_flash_time: ResMut<CursorFlashTime>,
//     inputs: Res<ButtonInput<KeyCode>>,
//     mut evr_keyboard: MessageReader<KeyboardInput>,
// ) {
//     let Some(focused_entity) = focused.0 else {
//         evr_keyboard.clear();
//         return;
//     };
//
//     let Ok((mut value, mut text_input)) = q_text_inputs.get_mut(focused_entity) else {
//         evr_keyboard.clear();
//         return;
//     };
//
//     for pressed in evr_keyboard.read() {
//         if pressed.state != ButtonState::Pressed {
//             continue;
//         }
//
//         if inputs.pressed(KeyCode::ControlLeft) || inputs.pressed(KeyCode::ControlRight) {
//             match pressed.key_code {
//                 KeyCode::KeyA => {
//                     if !value.is_empty() {
//                         text_input.highlight_begin = Some(0);
//                         text_input.cursor_pos = value.len();
//                     }
//
//                     cursor_flash_time.0 = 0.0;
//                 }
//                 KeyCode::KeyC => {
//                     let Ok(mut clipboard) = Clipboard::new() else {
//                         continue;
//                     };
//
//                     let Some(range) = text_input.get_highlighted_range() else {
//                         continue;
//                     };
//
//                     if let Err(err) = clipboard.set_text(&value[range]) {
//                         warn!("{err}");
//                     }
//                 }
//                 KeyCode::KeyX => {
//                     let Ok(mut clipboard) = Clipboard::new() else {
//                         continue;
//                     };
//
//                     let Some(range) = text_input.get_highlighted_range() else {
//                         continue;
//                     };
//
//                     if let Err(err) = clipboard.set_text(&value[range.clone()]) {
//                         warn!("{err}");
//                     }
//
//                     let mut new_value = value.0.clone();
//                     let new_cursor_pos = range.start;
//                     new_value.replace_range(range, "");
//
//                     if verify_input(&text_input, &new_value) {
//                         value.0 = new_value;
//                         text_input.cursor_pos = new_cursor_pos;
//                         text_input.highlight_begin = None;
//                     }
//                 }
//                 KeyCode::KeyV => {
//                     let Ok(mut clipboard) = Clipboard::new() else {
//                         continue;
//                     };
//
//                     let Ok(clipboard_contents) = clipboard.get_text() else {
//                         continue;
//                     };
//
//                     let mut new_value = value.0.clone();
//                     let new_cursor_pos;
//
//                     if let Some(range) = text_input.get_highlighted_range() {
//                         let replace_string = clipboard_contents;
//                         new_cursor_pos = range.start + replace_string.len();
//
//                         new_value.replace_range(range, &replace_string);
//                     } else if text_input.cursor_pos == value.len() {
//                         new_value.push_str(&clipboard_contents);
//
//                         new_cursor_pos = clipboard_contents.len() + text_input.cursor_pos;
//                     } else {
//                         new_value.insert_str(text_input.cursor_pos, &clipboard_contents);
//
//                         new_cursor_pos = clipboard_contents.len() + text_input.cursor_pos;
//                     }
//
//                     if verify_input(&text_input, &new_value) {
//                         value.0 = new_value;
//                         text_input.cursor_pos = new_cursor_pos;
//                         text_input.highlight_begin = None;
//                     }
//                 }
//                 KeyCode::ArrowLeft => {
//                     if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
//                         text_input.highlight_begin = Some(text_input.cursor_pos);
//                     } else {
//                         text_input.highlight_begin = None;
//                     }
//                     text_input.cursor_pos = 0;
//                     cursor_flash_time.0 = 0.0;
//                 }
//                 KeyCode::ArrowRight => {
//                     if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
//                         text_input.highlight_begin = Some(text_input.cursor_pos);
//                     } else {
//                         text_input.highlight_begin = None;
//                     }
//                     text_input.cursor_pos = value.len();
//                     cursor_flash_time.0 = 0.0;
//                 }
//                 _ => {}
//             }
//         } else {
//             match pressed.key_code {
//                 KeyCode::ArrowLeft => {
//                     if text_input.cursor_pos != 0 {
//                         if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
//                             if text_input.highlight_begin.is_none() {
//                                 text_input.highlight_begin = Some(text_input.cursor_pos);
//                             }
//                         } else {
//                             text_input.highlight_begin = None;
//                         }
//                         text_input.cursor_pos -= 1;
//                     }
//                     cursor_flash_time.0 = 0.0;
//                 }
//                 KeyCode::ArrowRight => {
//                     if text_input.cursor_pos != value.len() {
//                         if inputs.pressed(KeyCode::ShiftLeft) || inputs.pressed(KeyCode::ShiftRight) {
//                             if text_input.highlight_begin.is_none() {
//                                 text_input.highlight_begin = Some(text_input.cursor_pos);
//                             }
//                         } else {
//                             text_input.highlight_begin = None;
//                         }
//                         text_input.cursor_pos += 1;
//                     }
//                     cursor_flash_time.0 = 0.0;
//                 }
//                 _ => {}
//             }
//         }
//     }
//
//     if let Some(highlighted) = text_input.highlight_begin
//         && highlighted == text_input.cursor_pos
//     {
//         text_input.highlight_begin = None;
//     }
// }
//
// fn verify_input(text_input: &TextInput, test_value: &str) -> bool {
//     match text_input.input_type {
//         InputType::Text { max_length } => max_length.map(|max_len| test_value.len() <= max_len).unwrap_or(true),
//         InputType::Integer { min, max } => test_value == "-" || test_value.parse::<i64>().map(|x| x >= min && x <= max).unwrap_or(false),
//         InputType::Decimal { min, max } => test_value == "-" || test_value.parse::<f64>().map(|x| x >= min && x <= max).unwrap_or(false),
//         // InputType::Custom(check) => check(test_value),
//     }
// }

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the TextInput component uses. Make sure you add any [`TextInput`] components before this set!
pub enum TextInputUiSystemSet {
    /// Make sure you add any [`TextInput`] components before this set!
    ///
    /// Sets up any [`TextInput`] components added.
    AddTextInputBundle,
    /// Updates the slider to contain any values that have been set via the "react" system
    HandleReactValues,
    /// Updates any components based on the value being changed in this [`TextInput`]
    ///
    /// The results of this can be read in [`InputValue`].
    ValueChanged,
}

fn on_click_text_input(mut click: On<Pointer<Click>>, q_text_input: Query<Entity, With<TextInputNode>>, mut focused: ResMut<InputFocus>) {
    let not_handled = if q_text_input.contains(click.entity) {
        focused.0 = Some(click.entity);
        false
    } else {
        true
    };

    click.propagate(not_handled);
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            TextInputUiSystemSet::AddTextInputBundle,
            TextInputUiSystemSet::HandleReactValues,
            TextInputUiSystemSet::ValueChanged,
        )
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            added_text_input_bundle.in_set(TextInputUiSystemSet::AddTextInputBundle),
            value_changed.in_set(TextInputUiSystemSet::ValueChanged),
        ),
    )
    .add_observer(on_click_text_input)
    .register_type::<TextInput>()
    .register_type::<InputValue>()
    .init_resource::<CursorFlashTime>();
}
