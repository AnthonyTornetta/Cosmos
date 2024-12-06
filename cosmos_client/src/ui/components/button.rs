//! An batteries included way to add an interactive button that will easily send out events when a button is clicked.

use std::marker::PhantomData;

use bevy::prelude::*;
use cosmos_core::ecs::NeedsDespawned;

use crate::ui::UiSystemSet;

use super::Disabled;

/// An event that will be created and sent when a button is interacted with
pub trait ButtonEvent: Sized + Event + std::fmt::Debug {
    /// Create an instance of this event
    fn create_event(btn_entity: Entity) -> Self;
}

#[derive(Component, Debug)]
#[require(Node)]
/// A UI element that will send out events (of type `T`) when it is pressed.
///
/// This does NOT use the default bevy `Button` component.
pub struct Button<T: ButtonEvent> {
    /// boo
    pub _phantom: PhantomData<T>,
    /// Interaction state of the button.
    pub last_interaction: Interaction,
    /// Out-of-the-box color changing for the different
    /// states a button can be in. Leave `None` if you don't want this.
    pub button_styles: Option<ButtonStyles>,
    /// Text to display in the button. The text will be center aligned.
    pub text: Option<(String, TextFont, TextColor)>,
    /// Image to display in the button. The image will take up the entire button.
    pub image: Option<ImageNode>,
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

impl<T: ButtonEvent> Default for Button<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
            button_styles: Default::default(),
            last_interaction: Default::default(),
            text: Default::default(),
            image: Default::default(),
        }
    }
}

#[derive(Component)]
struct ButtonText(Entity);

fn on_add_button<T: ButtonEvent>(mut commands: Commands, mut q_added_button: Query<(Entity, &Button<T>, &mut Node), Added<Button<T>>>) {
    for (ent, button, mut style) in q_added_button.iter_mut() {
        commands.entity(ent).insert(Interaction::default());

        // horizontally + vertically center child text
        style.justify_content = JustifyContent::Center;
        style.align_items = AlignItems::Center;

        if let Some(ui_node) = button.image.clone() {
            commands.entity(ent).insert(ui_node);
        }

        if let Some((text, text_style)) = button.text.clone() {
            let text_ent = commands
                .spawn((
                    Name::new("Button Text"),
                    TextBundle {
                        text: Text::from_section(text, text_style),
                        ..Default::default()
                    },
                ))
                .id();

            commands.entity(ent).insert(ButtonText(text_ent)).add_child(text_ent);
        }
    }
}

fn on_interact_button<T: ButtonEvent>(
    mut ev_writer: EventWriter<T>,
    mut q_added_button: Query<
        (Entity, &Interaction, &mut Button<T>, &mut BackgroundColor, &Children),
        (Changed<Interaction>, Without<Disabled>),
    >,
    mut q_text: Query<&mut Text>,
) {
    for (btn_entity, interaction, mut button, mut bg_color, children) in q_added_button.iter_mut() {
        if let Some(btn_styles) = &button.button_styles {
            bg_color.0 = match *interaction {
                Interaction::None => btn_styles.background_color,
                Interaction::Hovered => btn_styles.hover_background_color,
                Interaction::Pressed => btn_styles.press_background_color,
            };

            if let Some(&text_child) = children.iter().find(|&x| q_text.contains(*x)) {
                let mut text = q_text.get_mut(text_child).expect("Checked above");

                let color = match *interaction {
                    Interaction::None => btn_styles.foreground_color,
                    Interaction::Hovered => btn_styles.hover_foreground_color,
                    Interaction::Pressed => btn_styles.press_foreground_color,
                };

                text.sections.iter_mut().for_each(|x| x.style.color = color);
            }
        }

        if *interaction == Interaction::Hovered && button.last_interaction == Interaction::Pressed {
            // Click and still hovering the button, so they didn't move out while holding the mouse down,
            // which should cancel the mouse click
            ev_writer.send(T::create_event(btn_entity));
        }

        button.last_interaction = *interaction;
    }
}

fn on_change_button<T: ButtonEvent>(
    mut commands: Commands,
    mut q_text: Query<&mut Text>,
    mut q_changed_button: Query<
        (
            Entity,
            Ref<Button<T>>,
            Option<&ImageNode>,
            Option<&ButtonText>,
            &Interaction,
            &mut BackgroundColor,
        ),
        Changed<Button<T>>,
    >,
) {
    for (ent, btn, image, button_text, &interaction, mut bg_color) in q_changed_button.iter_mut().filter(|x| !x.1.is_added()) {
        fn calc_text_color<T: ButtonEvent>(btn: &Button<T>, interaction: Interaction, text_style: &mut TextColor) {
            if let Some(btn_styles) = &btn.button_styles {
                text_style.color = match interaction {
                    Interaction::None => btn_styles.foreground_color,
                    Interaction::Hovered => btn_styles.hover_foreground_color,
                    Interaction::Pressed => btn_styles.press_foreground_color,
                };
            }
        }

        if !image
            .map(|x| {
                if let Some(y) = &btn.image {
                    x.flip_x == y.flip_x && x.flip_y == y.flip_y && x.texture == y.texture
                } else {
                    false
                }
            })
            .unwrap_or(btn.image.is_none())
        {
            if let Some(image) = btn.image.clone() {
                commands.entity(ent).insert(image);
            } else {
                commands.entity(ent).remove::<ImageNode>();
            }
        }

        if let Some(button_text) = button_text {
            if let Some((new_text_value, text_style)) = &btn.text {
                let Ok(mut text) = q_text.get_mut(button_text.0) else {
                    error!("Text entity has no text");
                    continue;
                };

                if let Some(text_section) = text.sections.first() {
                    let same_text_style = text_section.style.color == text_style.color
                        && text_section.style.font_size == text_style.font_size
                        && text_section.style.font == text_style.font;

                    if !same_text_style || &text_section.value != new_text_value {
                        let mut text_style = text_style.clone();
                        calc_text_color(&btn, interaction, &mut text_style);
                        text.sections = vec![TextSection::new(new_text_value.clone(), text_style)];
                    }
                } else {
                    let mut text_style = text_style.clone();
                    calc_text_color(&btn, interaction, &mut text_style);
                    text.sections = vec![TextSection::new(new_text_value.clone(), text_style.clone())];
                }
            } else {
                commands.entity(button_text.0).insert(NeedsDespawned);
                commands.entity(ent).remove::<ButtonText>();
            }
        } else if let Some((text, mut text_style)) = btn.text.clone() {
            calc_text_color(&btn, interaction, &mut text_style);

            let text_ent = commands
                .spawn((
                    Name::new("Button Text"),
                    TextBundle {
                        text: Text::from_section(text, text_style),
                        ..Default::default()
                    },
                ))
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

/// When you make a new [`ButtonEvent`] type and add a button, you must call this method or they will not work.
pub fn register_button<T: ButtonEvent>(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_button::<T>.in_set(ButtonUiSystemSet::AddButtonBundle),
            on_change_button::<T>.in_set(ButtonUiSystemSet::SendButtonEvents),
            on_interact_button::<T>.in_set(ButtonUiSystemSet::SendButtonEvents),
        )
            .chain(),
    )
    .add_event::<T>();
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (ButtonUiSystemSet::AddButtonBundle, ButtonUiSystemSet::SendButtonEvents)
            .chain()
            .in_set(UiSystemSet::DoUi),
    );
}
