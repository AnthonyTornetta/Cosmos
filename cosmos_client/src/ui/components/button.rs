//! An batteries included way to add an interactive button that will easily send out events when a button is clicked.

use bevy::color::palettes::css;
use bevy::prelude::*;

use cosmos_core::ecs::NeedsDespawned;

use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};
use crate::ui::UiSystemSet;

use super::Disabled;
use super::show_cursor::any_open_menus;

/// An event that will be created and sent when a button is interacted with
#[derive(Event, Debug)]
#[event(traversal = &'static ChildOf, auto_propagate)]
pub struct ButtonEvent(pub Entity);

#[derive(Component, Debug, Default)]
#[require(Node)]
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
        commands.entity(ent).insert(Interaction::default());

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
                .spawn((Name::new("Button Text"), Text::new(text), text_style, text_color))
                .id();

            commands.entity(ent).insert(ButtonText(text_ent)).add_child(text_ent);
        }
    }
}

fn on_interact_button(
    mut q_added_button: Query<
        (Entity, &Interaction, &mut CosmosButton, &mut BackgroundColor, Option<&Children>),
        (Changed<Interaction>, Without<Disabled>),
    >,
    mut writer: TextUiWriter,
    q_has_text: Query<(), With<Text>>,
    mut commands: Commands,
    inputs: InputChecker,
) {
    for (btn_entity, interaction, mut button, mut bg_color, children) in q_added_button.iter_mut() {
        if let Some(btn_styles) = &button.button_styles {
            bg_color.0 = match *interaction {
                Interaction::None => btn_styles.background_color,
                Interaction::Hovered => btn_styles.hover_background_color,
                Interaction::Pressed => btn_styles.press_background_color,
            };

            if let Some(children) = children
                && let Some(text_child) = children.iter().find(|&x| q_has_text.contains(x))
            {
                let color = match *interaction {
                    Interaction::None => btn_styles.foreground_color,
                    Interaction::Hovered => btn_styles.hover_foreground_color,
                    Interaction::Pressed => btn_styles.press_foreground_color,
                };

                writer.for_each_color(text_child, |mut c| c.0 = color);
            }
        }

        if button.submit_control.map(|c| inputs.check_just_pressed(c)).unwrap_or(true)
            || (*interaction == Interaction::Hovered && button.last_interaction == Interaction::Pressed)
        {
            // Click and still hovering the button, so they didn't move out while holding the mouse down,
            // which should cancel the mouse click
            commands.entity(btn_entity).trigger(ButtonEvent(btn_entity));
        }

        button.last_interaction = *interaction;
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
            &Interaction,
            &mut BackgroundColor,
        ),
        Changed<CosmosButton>,
    >,
    mut writer: TextUiWriter,
) {
    for (ent, btn, image, button_text, &interaction, mut bg_color) in q_changed_button.iter_mut().filter(|x| !x.1.is_added()) {
        fn calc_text_color(btn: &CosmosButton, interaction: Interaction, text_style: &mut TextColor) {
            if let Some(btn_styles) = &btn.button_styles {
                text_style.0 = match interaction {
                    Interaction::None => btn_styles.foreground_color,
                    Interaction::Hovered => btn_styles.hover_foreground_color,
                    Interaction::Pressed => btn_styles.press_foreground_color,
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
                Interaction::None => btn_styles.background_color,
                Interaction::Hovered => btn_styles.hover_background_color,
                Interaction::Pressed => btn_styles.press_background_color,
            }
            .into();
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
    SendButtonEvents,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (ButtonUiSystemSet::AddButtonBundle, ButtonUiSystemSet::SendButtonEvents)
            .chain()
            .in_set(UiSystemSet::DoUi),
    )
    .add_systems(
        Update,
        (
            on_add_button.in_set(ButtonUiSystemSet::AddButtonBundle),
            on_change_button.in_set(ButtonUiSystemSet::SendButtonEvents),
            on_interact_button
                .in_set(ButtonUiSystemSet::SendButtonEvents)
                .run_if(any_open_menus),
        )
            .chain(),
    )
    .add_event::<ButtonEvent>();
}
