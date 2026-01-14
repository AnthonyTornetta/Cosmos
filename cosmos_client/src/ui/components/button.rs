//! An batteries included way to add an interactive button that will easily send out events when a button is clicked.

use bevy::color::palettes::css;
use bevy::picking::hover::PickingInteraction;
use bevy::prelude::*;

use cosmos_core::ecs::NeedsDespawned;

use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};
use crate::ui::UiSystemSet;

use super::Disabled;
use super::show_cursor::any_open_menus;

/// An event that will be created and sent when a button is interacted with
#[derive(EntityEvent, Debug)]
#[entity_event(propagate = &'static ChildOf)]
pub struct ButtonEvent(pub Entity);

#[derive(Component, Debug, Default)]
#[require(Node, Pickable)]
/// A UI element that will send out events (of type `T`) when it is pressed.
///
/// This does NOT use the default bevy `Button` component.
pub struct CosmosButton {
    /// Interaction state of the button.
    pub last_interaction: Interaction,
    /// Out-of-the-box color changing for the different
    /// states a button can be in. Leave `None` if you don't want this.
    pub button_styles: Option<ButtonStyles>,
    /// Text to display in the button. The text will be center aligned.
    pub text: Option<(String, TextFont, TextColor)>,
    /// Image to display in the button. The image will take up the entire button.
    pub image: Option<ImageNode>,
    /// Will treat it as a click when this key is pressed
    pub submit_control: Option<CosmosInputs>,
}

#[derive(Debug, Clone)]
/// Out-of-the-box color changing for the different
/// states a button can be in.
pub struct ButtonStyles {
    /// Color used when not hovering/clicking the button
    pub background_color: Color,
    /// Color used when not hovering/clicking the button
    pub foreground_color: Color,

    /// Color used when hovering but not clicking the button
    pub hover_background_color: Color,
    /// Color used when hovering but not clicking the button
    pub hover_foreground_color: Color,

    /// Color used when clicking the button
    pub press_background_color: Color,
    /// Color used when clicking the button
    pub press_foreground_color: Color,
}

impl Default for ButtonStyles {
    fn default() -> Self {
        Self {
            background_color: css::GRAY.into(),
            foreground_color: css::WHITE.into(),
            hover_background_color: css::GRAY.into(),
            hover_foreground_color: css::WHITE.into(),
            press_background_color: Srgba::hex("333333").unwrap().into(),
            press_foreground_color: css::WHITE.into(),
        }
    }
}

#[derive(Component)]
struct ButtonText(Entity);

fn on_add_button(mut commands: Commands, mut q_added_button: Query<(Entity, &CosmosButton, &mut Node), Added<CosmosButton>>) {
    for (ent, button, mut style) in q_added_button.iter_mut() {
        commands.entity(ent).insert(PickingInteraction::default());

        if let Some(bg_col) = button.button_styles.as_ref().map(|x| x.background_color) {
            commands.entity(ent).insert(BackgroundColor(bg_col));
        }

        // horizontally + vertically center child text
        style.justify_content = JustifyContent::Center;
        style.align_items = AlignItems::Center;

        if let Some(ui_node) = button.image.clone() {
            commands.entity(ent).insert(ui_node);
        }

        if let Some((text, text_style, text_color)) = button.text.clone() {
            let text_ent = commands
                .spawn((
                    Name::new("Button Text"),
                    Pickable {
                        should_block_lower: false,
                        is_hoverable: false,
                    },
                    Text::new(text),
                    text_style,
                    text_color,
                ))
                .id();

            commands.entity(ent).insert(ButtonText(text_ent)).add_child(text_ent);
        }
    }
}

fn on_interact_button_keybind(
    mut q_added_button: Query<(Entity, &CosmosButton), Without<Disabled>>,
    mut commands: Commands,
    inputs: InputChecker,
) {
    for (btn_entity, button) in q_added_button.iter_mut() {
        if let Some(submit_control) = button.submit_control
            && inputs.check_just_pressed(submit_control)
        {
            commands.entity(btn_entity).trigger(ButtonEvent);
        }
    }
}

fn on_change_button(
    mut commands: Commands,
    mut q_changed_button: Query<
        (
            Entity,
            Ref<CosmosButton>,
            Option<&ImageNode>,
            Option<&ButtonText>,
            &PickingInteraction,
            &mut BackgroundColor,
        ),
        Changed<CosmosButton>,
    >,
    mut writer: TextUiWriter,
) {
    for (ent, btn, image, button_text, &interaction, mut bg_color) in q_changed_button.iter_mut().filter(|x| !x.1.is_added()) {
        fn calc_text_color(btn: &CosmosButton, interaction: PickingInteraction, text_style: &mut TextColor) {
            if let Some(btn_styles) = &btn.button_styles {
                text_style.0 = match interaction {
                    PickingInteraction::None => btn_styles.foreground_color,
                    PickingInteraction::Hovered => btn_styles.hover_foreground_color,
                    PickingInteraction::Pressed => btn_styles.press_foreground_color,
                };
            }
        }

        if !image
            .map(|x| {
                if let Some(y) = &btn.image {
                    x.flip_x == y.flip_x && x.flip_y == y.flip_y && x.image == y.image
                } else {
                    false
                }
            })
            .unwrap_or(btn.image.is_none())
            && let Some(image) = btn.image.clone()
        {
            commands.entity(ent).insert(image);
        }

        if let Some(button_text) = button_text {
            if let Some((new_text_value, text_font, text_color)) = &btn.text {
                if let Some((_, _, cur_text_value, cur_font_style, cur_text_color)) = writer.get(button_text.0, 0) {
                    let same_text_style = text_color.0 == cur_text_color.as_ref().0
                        && text_font.font == cur_font_style.font
                        && text_font.font_size == cur_font_style.font_size
                        && text_font.font_smoothing == cur_font_style.font_smoothing;

                    if !same_text_style || new_text_value != cur_text_value.as_ref() {
                        writer.color(button_text.0, 0).as_mut().0 = text_color.0;
                        *writer.font(button_text.0, 0).as_mut() = text_font.clone();
                        *writer.text(button_text.0, 0).as_mut() = new_text_value.clone();
                    }
                } else {
                    error!("It happened!!!");
                    // let mut col = text_color.clone();
                    // let font = text_font.clone();
                    // calc_text_color(&btn, interaction, &mut col);
                    //
                    // writer.text.sections = vec![TextSection::new(new_text_value.clone(), text_style.clone())];
                }
            } else {
                commands.entity(button_text.0).insert(NeedsDespawned);
                commands.entity(ent).remove::<ButtonText>();
            }
        } else if let Some((text, text_style, mut text_color)) = btn.text.clone() {
            calc_text_color(&btn, interaction, &mut text_color);

            let text_ent = commands
                .spawn((Name::new("Button Text"), Text::new(text), text_style, text_color))
                .id();

            commands.entity(ent).insert(ButtonText(text_ent)).add_child(text_ent);
        }

        if let Some(btn_styles) = &btn.button_styles {
            *bg_color = match interaction {
                PickingInteraction::None => btn_styles.background_color,
                PickingInteraction::Hovered => btn_styles.hover_background_color,
                PickingInteraction::Pressed => btn_styles.press_background_color,
            }
            .into();
        }
    }
}

fn on_click(mut click: On<Pointer<Click>>, q_btn: Query<(), (With<CosmosButton>, Without<Disabled>)>, mut commands: Commands) {
    if !q_btn.contains(click.entity) {
        return;
    };

    commands.entity(click.entity).trigger(|_| ButtonEvent(click.entity));

    click.propagate(false);
}

fn on_over(
    mut over: On<Pointer<Over>>,
    mut q_btn: Query<(&CosmosButton, Option<&Children>, &mut BackgroundColor), Without<Disabled>>,
    mut writer: TextUiWriter,
    q_has_text: Query<(), With<Text>>,
) {
    let Ok((button, children, mut bg_color)) = q_btn.get_mut(over.entity) else {
        return;
    };

    over.propagate(false);

    if let Some(btn_styles) = &button.button_styles {
        bg_color.0 = btn_styles.hover_background_color;

        if let Some(children) = children
            && let Some(text_child) = children.iter().find(|&x| q_has_text.contains(x))
        {
            let color = btn_styles.hover_foreground_color;

            writer.for_each_color(text_child, |mut c| c.0 = color);
        }
    }
}

fn on_out(
    mut out: On<Pointer<Out>>,
    mut q_btn: Query<(&CosmosButton, Option<&Children>, &mut BackgroundColor), Without<Disabled>>,
    mut writer: TextUiWriter,
    q_has_text: Query<(), With<Text>>,
) {
    let Ok((button, children, mut bg_color)) = q_btn.get_mut(out.entity) else {
        return;
    };

    out.propagate(false);

    if let Some(btn_styles) = &button.button_styles {
        bg_color.0 = btn_styles.background_color;

        if let Some(children) = children
            && let Some(text_child) = children.iter().find(|&x| q_has_text.contains(x))
        {
            let color = btn_styles.foreground_color;

            writer.for_each_color(text_child, |mut c| c.0 = color);
        }
    }
}

fn on_press(
    mut press: On<Pointer<Press>>,
    mut q_btn: Query<(&CosmosButton, Option<&Children>, &mut BackgroundColor), Without<Disabled>>,
    mut writer: TextUiWriter,
    q_has_text: Query<(), With<Text>>,
) {
    let Ok((button, children, mut bg_color)) = q_btn.get_mut(press.entity) else {
        return;
    };

    press.propagate(false);

    if let Some(btn_styles) = &button.button_styles {
        bg_color.0 = btn_styles.press_background_color;

        if let Some(children) = children
            && let Some(text_child) = children.iter().find(|&x| q_has_text.contains(x))
        {
            let color = btn_styles.press_foreground_color;

            writer.for_each_color(text_child, |mut c| c.0 = color);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the [`Button`]` component uses. Make sure you add any [`Button`] components before this set!
pub enum ButtonUiSystemSet {
    /// Make sure you add any [`Button`] components before this set!
    ///
    /// Sets up any [`Button`] components added.
    AddButtonBundle,
    /// Sends user events from the various [`Button`] components.
    SendButtonMessages,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (ButtonUiSystemSet::AddButtonBundle, ButtonUiSystemSet::SendButtonMessages)
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            on_add_button.in_set(ButtonUiSystemSet::AddButtonBundle),
            on_change_button.in_set(ButtonUiSystemSet::SendButtonMessages),
            on_interact_button_keybind
                .in_set(ButtonUiSystemSet::SendButtonMessages)
                .run_if(any_open_menus),
        )
            .chain(),
    )
    .add_observer(on_click)
    .add_observer(on_press)
    .add_observer(on_over)
    .add_observer(on_out);
}
