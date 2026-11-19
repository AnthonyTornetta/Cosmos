//! A collection of generic UI elements that can be used

use bevy::{
    input::keyboard::{Key, KeyboardInput},
    input_focus::{FocusedInput, InputFocus},
    picking::hover::PickingInteraction,
    prelude::*,
    text::{EditableText, EditableTextSystems, LineHeight, TextCursorStyle, TextEdit, TextReadWriteMode},
    ui::{FocusPolicy, InteractionDisabled, UiSystems},
    ui_widgets::TextInput as TextInputNode,
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

#[derive(Component)]
struct InputPlaceholder(Entity);

#[derive(Component, Default)]
struct InputHistory {
    undo: Vec<String>,
    redo: Vec<String>,
    replaying: bool,
}

// Bevy 0.20 handles Key::Space only without modifiers. Supplement that mapping
// for Shift+Space, leaving character input and editing shortcuts to Bevy.
fn shifted_space(
    mut input: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<Key>>,
    mut query: Query<(&mut EditableText, &TextReadWriteMode, Has<InteractionDisabled>), With<InputHistory>>,
) {
    if input.input.logical_key != Key::Space
        || !input.input.state.is_pressed()
        || !keys.pressed(Key::Shift)
        || keys.any_pressed([Key::Control, Key::Super, Key::Alt])
    {
        return;
    }
    let Ok((mut text, mode, disabled)) = query.get_mut(input.focused_entity) else {
        return;
    };
    if *mode == TextReadWriteMode::Editable
        && !disabled
        && !text.is_composing()
        && let Some(value) = &input.input.text
        && !value.is_empty()
    {
        text.queue_edit(TextEdit::Insert(value.clone()));
    }
    input.propagate(false);
}

fn undo_redo(
    mut input: On<FocusedInput<KeyboardInput>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut EditableText, &mut InputHistory)>,
) {
    if !input.input.state.is_pressed()
        || !(keys.pressed(KeyCode::ControlLeft)
            || keys.pressed(KeyCode::ControlRight)
            || keys.pressed(KeyCode::SuperLeft)
            || keys.pressed(KeyCode::SuperRight))
    {
        return;
    }
    let Key::Character(key) = &input.input.logical_key else {
        return;
    };
    let redo = key.eq_ignore_ascii_case("y")
        || (key.eq_ignore_ascii_case("z") && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)));
    if !redo && !key.eq_ignore_ascii_case("z") {
        return;
    }
    let Ok((mut text, mut history)) = query.get_mut(input.focused_entity) else {
        return;
    };
    let value = if redo { history.redo.pop() } else { history.undo.pop() };
    if let Some(value) = value {
        let current = text.value().to_string();
        if redo {
            history.undo.push(current);
        } else {
            history.redo.push(current);
        }
        history.replaying = true;
        text.queue_edit(TextEdit::SelectAll);
        text.queue_edit(TextEdit::Insert(value.into()));
    }
    input.propagate(false);
}

fn calc_line_height(computed_node: &ComputedNode) -> f32 {
    let border = computed_node.border();
    computed_node.size().y
        - (border.max_inset.y + border.min_inset.y)
        - (computed_node.padding.max_inset.y + computed_node.padding.min_inset.y)
}

fn update_line_height(
    q_text_input: Query<(&Children, &ComputedNode), (With<TextInput>, Changed<ComputedNode>)>,
    mut q_line_height: Query<&mut LineHeight>,
) {
    for (children, node) in &q_text_input {
        for child in children.iter() {
            if let Ok(mut line_height) = q_line_height.get_mut(child) {
                *line_height = LineHeight::Px(calc_line_height(node));
            }
        }
    }
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
    for (entity, mut node, computed_node, text_layout, input_value, font, color, input, placeholder) in &mut q_added {
        if node.height == Val::Auto {
            node.height = Val::Px(40.0 + computed_node.padding.max_inset.y + computed_node.padding.min_inset.y);
        }
        commands.entity(entity).insert(FocusPolicy::Block);
        let max_characters = match input.input_type {
            InputType::Text { max_length } => max_length,
            _ => None,
        };
        let text_entity = commands
            .spawn((
                TextInputNode,
                InputHistory::default(),
                EditableText {
                    max_characters,
                    ..EditableText::new(input_value.value())
                },
                font.clone(),
                *color,
                TextCursorStyle {
                    color: color.0,
                    ..default()
                },
                LineHeight::Px(calc_line_height(computed_node)),
                text_layout.copied().unwrap_or_default(),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..input.text_node.clone()
                },
                Pickable::default(),
                PickingInteraction::default(),
                FocusPolicy::Block,
                Name::new("Text input"),
            ))
            .id();
        commands.entity(entity).add_child(text_entity).insert(TextEnt(text_entity));
        if let Some(placeholder) = placeholder {
            let placeholder_entity = commands
                .spawn((
                    Text::new(&placeholder.text),
                    font.clone(),
                    TextColor(placeholder.color),
                    LineHeight::Px(calc_line_height(computed_node)),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Pickable::IGNORE,
                    InputPlaceholder(text_entity),
                ))
                .id();
            commands.entity(entity).add_child(placeholder_entity);
        }
    }
}

fn changed_text_input(q_text_input: Query<(&TextEnt, &TextInput), Changed<TextInput>>, mut q_text: Query<&mut EditableText>) {
    for (entity, input) in &q_text_input {
        if let Ok(mut text) = q_text.get_mut(entity.0) {
            text.max_characters = match input.input_type {
                InputType::Text { max_length } => max_length,
                _ => None,
            };
        }
    }
}

fn set_text(text: &mut EditableText, value: &str) {
    text.clear();
    text.editor_mut().set_text(value);
    text.queue_edit(TextEdit::TextEnd(false));
}

fn value_changed(
    q_values_changed: Query<(&InputValue, &TextEnt), Or<(Changed<InputValue>, Changed<TextInput>)>>,
    mut q_text: Query<(&mut EditableText, &mut InputHistory)>,
) {
    for (value, entity) in &q_values_changed {
        if let Ok((mut text, mut history)) = q_text.get_mut(entity.0)
            && text.value() != value.value()
        {
            *history = InputHistory::default();
            set_text(&mut text, value.value());
        }
    }
}

fn valid_input(input_type: InputType, value: &str) -> bool {
    match input_type {
        InputType::Text { max_length } => max_length.is_none_or(|max| value.chars().count() <= max),
        InputType::Integer { min, max } => {
            value.is_empty() || (value == "-" && min < 0) || value.parse::<i64>().is_ok_and(|number| number >= min && number <= max)
        }
        InputType::Decimal { min, max } => {
            value.is_empty() || (value == "-" && min < 0.0) || value.parse::<f64>().is_ok_and(|number| number >= min && number <= max)
        }
    }
}

fn on_change_text(
    mut q_text: Query<(&mut EditableText, &ChildOf, &mut InputHistory), Changed<EditableText>>,
    mut q_value: Query<(&mut InputValue, &TextInput)>,
) {
    for (mut text, parent, mut history) in &mut q_text {
        let Ok((mut value, input)) = q_value.get_mut(parent.parent()) else {
            continue;
        };
        let edited = text.value().to_string();
        if edited == value.value() {
            continue;
        }
        if valid_input(input.input_type, &edited) {
            if !history.replaying {
                history.undo.push(value.value().to_owned());
                history.redo.clear();
            }
            history.replaying = false;
            value.set_value(edited);
        } else {
            set_text(&mut text, value.value());
        }
    }
}

fn update_placeholders(q_text: Query<&EditableText>, mut q_placeholder: Query<(&InputPlaceholder, &mut Visibility)>) {
    for (placeholder, mut visibility) in &mut q_placeholder {
        let empty = q_text.get(placeholder.0).is_ok_and(|text| text.value().to_string().is_empty());
        *visibility = if empty { Visibility::Inherited } else { Visibility::Hidden };
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System sets used to create text fields and apply reactive values.
pub enum TextInputUiSystemSet {
    /// Add text input components before this set.
    AddTextInputBundle,
    /// Applies values supplied by the reactive UI.
    HandleReactValues,
}

fn on_click_text_input(mut click: On<PointerClick>, q_text: Query<(), With<TextInputNode>>, mut focus: ResMut<InputFocus>) {
    if q_text.contains(click.entity) {
        focus.set(click.entity, bevy::input_focus::FocusCause::Pressed);
        click.propagate(false);
    }
}

fn on_spawn_focus(
    q_ent: Query<&Children, Or<(Added<OnSpawnFocus>, Added<TextInput>)>>,
    mut focused: ResMut<InputFocus>,
    q_text: Query<(), With<TextInputNode>>,
) {
    for children in &q_ent {
        for child in children.iter() {
            if q_text.contains(child) {
                focused.set(child, bevy::input_focus::FocusCause::Auto);
                return;
            }
        }
    }
}

fn keep_focus(
    q_ent: Query<(&Children, &ComputedNode), (With<TextInput>, With<KeepFocused>)>,
    mut focused: ResMut<InputFocus>,
    q_text: Query<(), With<TextInputNode>>,
) {
    for (children, node) in &q_ent {
        if node.is_empty() {
            continue;
        }
        for child in children.iter() {
            if q_text.contains(child) {
                if focused.get() != Some(child) {
                    focused.set(child, bevy::input_focus::FocusCause::Auto);
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
        (
            added_text_input_bundle,
            on_spawn_focus,
            keep_focus,
            update_line_height,
            changed_text_input,
        )
            .chain()
            .in_set(TextInputUiSystemSet::AddTextInputBundle),
    )
    .add_systems(
        PostUpdate,
        (
            value_changed.before(EditableTextSystems),
            (on_change_text, update_placeholders)
                .chain()
                .after(EditableTextSystems)
                .before(UiSystems::PostLayout),
        ),
    )
    .add_observer(on_click_text_input)
    .add_observer(shifted_space)
    .add_observer(undo_redo)
    .register_type::<TextInput>()
    .register_type::<InputValue>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_fields_preserve_valid_editing_states_and_reject_out_of_range_values() {
        let integer = InputType::Integer { min: -10, max: 10 };
        for value in ["", "-", "-10", "0", "10"] {
            assert!(valid_input(integer, value), "{value}");
        }
        for value in ["-11", "11", "1.5", "hello"] {
            assert!(!valid_input(integer, value), "{value}");
        }
        let decimal = InputType::Decimal { min: -1.5, max: 1.5 };
        for value in ["", "-", "-1.5", "0.25", "1.5"] {
            assert!(valid_input(decimal, value), "{value}");
        }
        for value in ["-1.6", "1.6", "NaN", "inf"] {
            assert!(!valid_input(decimal, value), "{value}");
        }
        assert!(!valid_input(InputType::Integer { min: 0, max: 10 }, "-"));
        let text = InputType::Text { max_length: Some(2) };
        assert!(valid_input(text, "é界"));
        assert!(!valid_input(text, "é界a"));
    }

    #[test]
    fn text_input_schedule_is_compatible_with_bevy_ui() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ImagePlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
            bevy::ui_widgets::UiWidgetsPlugins,
        ));
        register(&mut app);
        app.world_mut().schedule_scope(PostUpdate, |world, schedule| {
            schedule.initialize(world).unwrap();
        });
    }

    #[test]
    fn text_input_creation_focuses_the_editor_and_preserves_placeholder_text() {
        let mut app = App::new();
        app.init_resource::<InputFocus>();
        register(&mut app);
        let parent = app
            .world_mut()
            .spawn((TextInput::default(), PlaceholderText::from("Name"), OnSpawnFocus))
            .id();
        app.update();
        let child = app.world().get::<TextEnt>(parent).unwrap().0;
        assert!(app.world().get::<TextInputNode>(child).is_some());
        assert_eq!(
            app.world().get::<TextCursorStyle>(child).unwrap().color,
            app.world().get::<TextColor>(child).unwrap().0
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(child));
        let placeholders: Vec<_> = app
            .world_mut()
            .query_filtered::<&Text, With<InputPlaceholder>>()
            .iter(app.world())
            .map(|text| text.0.clone())
            .collect();
        assert_eq!(placeholders, ["Name"]);
    }

    #[test]
    fn shifted_space_inserts_once_alongside_bevy_keyboard_handling() {
        let mut app = App::new();
        app.init_resource::<InputFocus>()
            .init_resource::<ButtonInput<Key>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(bevy::ui_widgets::TextInputPlugin);
        register(&mut app);
        let child = app.world_mut().spawn((TextInputNode, InputHistory::default())).id();

        for (shift, logical_key, key_code, value, repeat, expected) in [
            (false, Key::Space, KeyCode::Space, " ", false, TextEdit::Insert(" ".into())),
            (true, Key::Space, KeyCode::Space, " ", false, TextEdit::Insert(" ".into())),
            (true, Key::Space, KeyCode::Space, " ", true, TextEdit::Insert(" ".into())),
            (
                true,
                Key::Character("A".into()),
                KeyCode::KeyA,
                "A",
                false,
                TextEdit::Insert("A".into()),
            ),
            (
                true,
                Key::Character("!".into()),
                KeyCode::Digit1,
                "!",
                false,
                TextEdit::Insert("!".into()),
            ),
            (true, Key::ArrowLeft, KeyCode::ArrowLeft, "", false, TextEdit::Left(true)),
        ] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<Key>>();
            keys.reset_all();
            if shift {
                keys.press(Key::Shift);
            }
            app.world_mut().get_mut::<EditableText>(child).unwrap().pending_edits.clear();
            app.world_mut().trigger(FocusedInput::new(
                child,
                KeyboardInput {
                    key_code,
                    logical_key,
                    state: bevy::input::ButtonState::Pressed,
                    text: Some(value.into()),
                    repeat,
                    window: child,
                },
                child,
            ));
            assert_eq!(
                app.world().get::<EditableText>(child).unwrap().pending_edits.as_slice(),
                &[expected]
            );
        }

        for (modifier, state, mode, disabled) in [
            (
                Some(Key::Control),
                bevy::input::ButtonState::Pressed,
                TextReadWriteMode::Editable,
                false,
            ),
            (
                Some(Key::Super),
                bevy::input::ButtonState::Pressed,
                TextReadWriteMode::Editable,
                false,
            ),
            (
                Some(Key::Alt),
                bevy::input::ButtonState::Pressed,
                TextReadWriteMode::Editable,
                false,
            ),
            (None, bevy::input::ButtonState::Released, TextReadWriteMode::Editable, false),
            (None, bevy::input::ButtonState::Pressed, TextReadWriteMode::ReadOnly, false),
            (None, bevy::input::ButtonState::Pressed, TextReadWriteMode::Editable, true),
        ] {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<Key>>();
            keys.reset_all();
            keys.press(Key::Shift);
            if let Some(modifier) = modifier {
                keys.press(modifier);
            }
            app.world_mut().entity_mut(child).insert(mode);
            if disabled {
                app.world_mut().entity_mut(child).insert(InteractionDisabled);
            }
            app.world_mut().get_mut::<EditableText>(child).unwrap().pending_edits.clear();
            app.world_mut().trigger(FocusedInput::new(
                child,
                KeyboardInput {
                    key_code: KeyCode::Space,
                    logical_key: Key::Space,
                    state,
                    text: Some(" ".into()),
                    repeat: false,
                    window: child,
                },
                child,
            ));
            assert!(app.world().get::<EditableText>(child).unwrap().pending_edits.is_empty());
        }
    }

    #[test]
    fn editable_text_syncs_values_rejects_invalid_edits_and_updates_placeholders() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .add_observer(undo_redo)
            .add_systems(Update, (value_changed, on_change_text, update_placeholders).chain());
        let parent = app
            .world_mut()
            .spawn((
                TextInput {
                    input_type: InputType::Integer { min: 0, max: 10 },
                    ..default()
                },
                InputValue::new("5"),
            ))
            .id();
        let child = app
            .world_mut()
            .spawn((EditableText::default(), ChildOf(parent), InputHistory::default()))
            .id();
        app.world_mut().entity_mut(parent).insert(TextEnt(child));
        let placeholder = app.world_mut().spawn((InputPlaceholder(child), Visibility::Inherited)).id();
        app.update();
        assert_eq!(app.world().get::<EditableText>(child).unwrap().value(), "5");
        assert_eq!(*app.world().get::<Visibility>(placeholder).unwrap(), Visibility::Hidden);

        app.world_mut().get_mut::<EditableText>(child).unwrap().editor_mut().set_text("11");
        app.update();
        assert_eq!(app.world().get::<InputValue>(parent).unwrap().value(), "5");
        assert_eq!(app.world().get::<EditableText>(child).unwrap().value(), "5");

        app.world_mut().get_mut::<EditableText>(child).unwrap().editor_mut().set_text("7");
        app.update();
        assert_eq!(app.world().get::<InputValue>(parent).unwrap().value(), "7");

        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ControlLeft);
        for (key, expected) in [("z", "5"), ("y", "7")] {
            app.world_mut().trigger(FocusedInput::new(
                child,
                KeyboardInput {
                    key_code: if key == "z" { KeyCode::KeyZ } else { KeyCode::KeyY },
                    logical_key: Key::Character(key.into()),
                    state: bevy::input::ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window: child,
                },
                child,
            ));
            let text = app.world().get::<EditableText>(child).unwrap();
            assert!(text.pending_edits.contains(&TextEdit::Insert(expected.into())));
            // Simulate the edit application performed by Bevy's EditableTextSystems.
            set_text(&mut app.world_mut().get_mut::<EditableText>(child).unwrap(), expected);
            app.update();
            assert_eq!(app.world().get::<InputValue>(parent).unwrap().value(), expected);
            app.update();
        }

        app.world_mut().get_mut::<InputValue>(parent).unwrap().set_value("");
        app.update();
        assert_eq!(app.world().get::<EditableText>(child).unwrap().value(), "");
        assert_eq!(*app.world().get::<Visibility>(placeholder).unwrap(), Visibility::Inherited);
    }
}
