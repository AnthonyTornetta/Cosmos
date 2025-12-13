//! A collection of generic UI elements that can be used

use bevy::{input_focus::InputFocus, picking::hover::PickingInteraction, prelude::*, text::LineHeight, ui::FocusPolicy};
use bevy_ui_text_input::{
    TextInputBuffer, TextInputContents, TextInputFilter, TextInputMode, TextInputNode, TextInputPrompt, TextInputQueue,
    actions::{TextInputAction, TextInputEdit},
    edit::process_text_input_queues,
    text_input_pipeline::text_input_prompt_system,
};

use crate::ui::{
    UiSystemSet,
    components::focus::{KeepFocused, OnSpawnFocus},
};

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
    /// The node that the text will be placed onto
    pub text_node: Node,
}

#[derive(Debug, Clone, Reflect, Component)]
/// Placeholder text will show before any text is typed
pub struct PlaceholderText {
    /// The color this text should have (defaults to grey)
    pub color: Color,
    /// The text to display
    pub text: String,
}

impl From<&str> for PlaceholderText {
    fn from(value: &str) -> Self {
        Self {
            text: value.into(),
            ..Default::default()
        }
    }
}

impl Default for PlaceholderText {
    fn default() -> Self {
        Self {
            text: "".into(),
            color: bevy::color::palettes::css::GRAY.into(),
        }
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            input_type: InputType::Text { max_length: None },
            text_node: Node {
                align_self: AlignSelf::Center,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
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

fn update_line_height(
    q_text_input: Query<(&Children, &ComputedNode), (With<TextInput>, Changed<ComputedNode>)>,
    mut q_text_font: Query<&mut TextFont, With<TextInputNode>>,
) {
    for (children, node) in q_text_input.iter() {
        for child in children.iter() {
            let Ok(mut text_font) = q_text_font.get_mut(child) else {
                continue;
            };

            text_font.line_height = LineHeight::Px(calc_line_height(node));
        }
    }
}

fn calc_line_height(computed_node: &ComputedNode) -> f32 {
    let border = computed_node.border();

    computed_node.size().y - (border.top + border.bottom) - (computed_node.padding.top + computed_node.padding.bottom)
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
            Option<&PlaceholderText>,
        ),
        Added<TextInput>,
    >,
) {
    for (entity, mut node, computed_node, text_layout, input_value, t_font, t_col, ti, placeholder_text) in q_added.iter_mut() {
        if node.height == Val::Auto {
            // Auto doesn't work correctly
            node.height = Val::Px(40.0 + computed_node.padding.top + computed_node.padding.bottom);
        }

        commands.entity(entity).insert(Interaction::None).insert(FocusPolicy::Block);

        let mut text_ent = None;

        let height = calc_line_height(computed_node);

        commands.entity(entity).with_children(|p| {
            let max_chars = if let InputType::Text { max_length } = &ti.input_type {
                *max_length
            } else {
                None
            };

            let mut ecmds = p.spawn((
                TextFont {
                    line_height: LineHeight::Px(height),
                    ..t_font.clone()
                },
                TextInputContents::default(),
                TextInputNode {
                    mode: TextInputMode::SingleLine,
                    max_chars,
                    ..Default::default()
                },
                create_text_input_queue(input_value.value()),
                *t_col,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                },
                // ti.text_node.clone(),
                Pickable::default(),
                PickingInteraction::default(),
                FocusPolicy::Block,
                Name::new("Text input"),
            ));

            if let Some(placeholder_text) = placeholder_text {
                ecmds.insert(TextInputPrompt {
                    text: placeholder_text.text.clone(),
                    color: Some(placeholder_text.color),
                    ..Default::default()
                });
            }

            if let Some(text_layout) = text_layout {
                ecmds.insert(*text_layout);
            }

            match ti.input_type {
                InputType::Integer { min, max } => {
                    ecmds.insert(TextInputFilter::custom(move |s| {
                        if s.len() == 0 {
                            return true;
                        }

                        if s == "-" && min < 0 {
                            return true;
                        }
                        let Ok(num) = s.parse::<i64>() else {
                            return false;
                        };

                        num >= min && num <= max
                    }));
                }
                InputType::Decimal { min, max } => {
                    ecmds.insert(TextInputFilter::custom(move |s| {
                        if s.len() == 0 {
                            return true;
                        }

                        if s == "-" && min < 0.0 {
                            return true;
                        }
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

fn changed_text_input(
    q_text_input: Query<(&TextEnt, &TextInput), Changed<TextInput>>,
    mut commands: Commands,
    mut q_text_node: Query<&mut TextInputNode>,
) {
    for (text_ent, ti) in q_text_input.iter() {
        let mut ecmds = commands.entity(text_ent.0);

        match ti.input_type {
            InputType::Integer { min, max } => {
                ecmds.insert(TextInputFilter::custom(move |s| {
                    if s.len() == 0 {
                        return true;
                    }

                    if s == "-" && min < 0 {
                        return true;
                    }
                    let Ok(num) = s.parse::<i64>() else {
                        return false;
                    };

                    num >= min && num <= max
                }));
            }
            InputType::Decimal { min, max } => {
                ecmds.insert(TextInputFilter::custom(move |s| {
                    if s.len() == 0 {
                        return true;
                    }

                    if s == "-" && min < 0.0 {
                        return true;
                    }
                    let Ok(num) = s.parse::<f64>() else {
                        return false;
                    };

                    num >= min && num <= max
                }));
            }
            InputType::Text { max_length } => {
                let Ok(mut text_node) = q_text_node.get_mut(text_ent.0) else {
                    continue;
                };

                text_node.max_chars = max_length;
            }
        }
    }
}

fn value_changed(
    q_values_changed: Query<(&InputValue, &TextEnt), Or<(Changed<InputValue>, Changed<TextInput>)>>,
    q_text: Query<&TextInputContents>,
    mut commands: Commands,
) {
    for (input_val, text_ent) in q_values_changed.iter() {
        let Ok(text) = q_text.get(text_ent.0) else {
            continue;
        };

        if text.get() == input_val.value() {
            continue;
        }

        // let mut input_buffer = TextInputBuffer::default();
        // let editor = &mut input_buffer.editor;
        // // editor.set_selection(Selection::Line(Cursor {
        // //     line: 0,
        // //     index: 0,
        // //     affinity: Default::default(),
        // // }));
        // editor.insert_string(input_val.value(), None);

        commands.entity(text_ent.0).insert((
            // input_buffer,
            TextInputBuffer::default(),
            create_text_input_queue(input_val.value()),
            // TextInputQueue::default(),
            TextInputContents::default(),
        ));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the TextInput component uses. Make sure you add any [`TextInput`] components before this set!
pub enum TextInputUiSystemSet {
    /// Make sure you add any [`TextInput`] components before this set!
    ///
    /// Sets up any [`TextInput`] components added.
    AddTextInputBundle,
    /// Updates the text to contain any values that have been set via the "react" system
    HandleReactValues,
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

fn on_change_text(q_text: Query<(&TextInputContents, &ChildOf), Changed<TextInputContents>>, mut q_value: Query<&mut InputValue>) {
    for (contents, child_of) in q_text.iter() {
        let Ok(mut iv) = q_value.get_mut(child_of.parent()) else {
            continue;
        };

        iv.set_value(contents.get());
    }
}

fn on_spawn_focus(
    q_ent: Query<&Children, Or<(Added<OnSpawnFocus>, Added<TextInput>)>>,
    mut focused: ResMut<InputFocus>,
    q_text_input: Query<(), With<TextInputNode>>,
) {
    for children in q_ent.iter() {
        for child in children.iter() {
            if q_text_input.contains(child) {
                focused.set(child);
                return;
            }
        }
    }
}

fn keep_focus(
    q_ent: Query<(&Children, &ComputedNode), (With<TextInput>, With<KeepFocused>)>,
    mut focused: ResMut<InputFocus>,
    q_text_input: Query<(), With<TextInputNode>>,
) {
    for (children, comp_node) in q_ent.iter() {
        // This node is hidden
        if comp_node.is_empty() {
            continue;
        }
        for child in children.iter() {
            if q_text_input.contains(child) {
                if focused.0 != Some(child) {
                    focused.set(child);
                }
                return;
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (TextInputUiSystemSet::AddTextInputBundle, TextInputUiSystemSet::HandleReactValues)
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        ((
            added_text_input_bundle,
            on_spawn_focus,
            keep_focus,
            update_line_height,
            changed_text_input,
        )
            .chain()
            .in_set(TextInputUiSystemSet::AddTextInputBundle),),
    )
    .add_systems(
        // Ordering needs to align w/ systems present in https://github.com/ickshonpe/bevy_ui_text_input/blob/master/src/lib.rs#L63
        PostUpdate,
        (
            value_changed.before(process_text_input_queues),
            on_change_text.after(text_input_prompt_system),
        )
            .chain(),
    )
    .add_observer(on_click_text_input)
    .register_type::<TextInput>()
    .register_type::<InputValue>();
}
