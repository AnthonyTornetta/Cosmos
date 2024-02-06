use std::marker::PhantomData;

use bevy::{
    app::{App, Update},
    ecs::{
        bundle::Bundle,
        component::Component,
        entity::Entity,
        event::{Event, EventWriter},
        query::{Added, Changed},
        schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query},
    },
    hierarchy::{BuildChildren, Children},
    render::color::Color,
    text::{Text, TextAlignment, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        AlignSelf, BackgroundColor, Interaction, Style, Val,
    },
};

pub enum ButtonEventType {
    Click,
}

pub trait ButtonEvent: Sized + Event {
    fn create_event(event_type: ButtonEventType) -> Option<Self>;
}

#[derive(Component, Debug)]
pub struct Button<T: ButtonEvent> {
    pub _phantom: PhantomData<T>,
    pub last_interaction: Interaction,
    pub button_styles: Option<ButtonStyles>,
    pub starting_text: Option<(String, TextStyle)>,
}

#[derive(Default, Debug)]
pub struct ButtonStyles {
    pub background_color: Color,
    pub foreground_color: Color,

    pub hover_background_color: Color,
    pub hover_foreground_color: Color,

    pub press_background_color: Color,
    pub press_foreground_color: Color,
}

impl<T: ButtonEvent> Default for Button<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
            button_styles: Default::default(),
            last_interaction: Default::default(),
            starting_text: Default::default(),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct ButtonBundle<T: ButtonEvent> {
    /// The node bundle that will be used with the TextInput
    pub node_bundle: NodeBundle,
    pub button: Button<T>,
}

impl<T: ButtonEvent> Default for ButtonBundle<T> {
    fn default() -> Self {
        Self {
            button: Default::default(),
            node_bundle: Default::default(),
        }
    }
}

fn on_add_button<T: ButtonEvent>(mut commands: Commands, mut q_added_button: Query<(Entity, &mut Button<T>), Added<Button<T>>>) {
    for (ent, mut button) in q_added_button.iter_mut() {
        commands.entity(ent).insert(Interaction::default());

        if let Some((text, text_style)) = std::mem::take(&mut button.starting_text) {
            commands.entity(ent).with_children(|p| {
                p.spawn(TextBundle {
                    text: Text::from_section(text, text_style).with_alignment(TextAlignment::Center),
                    style: Style {
                        align_self: AlignSelf::Center,
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });
        }
    }
}

fn on_interact_button<T: ButtonEvent>(
    mut ev_writer: EventWriter<T>,
    mut q_added_button: Query<(&Interaction, &mut Button<T>, &mut BackgroundColor, &Children), Changed<Interaction>>,
    mut q_text: Query<&mut Text>,
) {
    for (interaction, mut button, mut bg_color, children) in q_added_button.iter_mut() {
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
            if let Some(ev) = T::create_event(ButtonEventType::Click) {
                ev_writer.send(ev);
            }
        }

        button.last_interaction = *interaction;
    }
}

// https://github.com/bevyengine/bevy/pull/9822
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System set the [`Button`]` component uses. Make sure you add any [`Button`] components before this set!
pub enum ButtonUiSystemSet {
    /// apply_deferred
    ApplyDeferredA,
    /// Make sure you add any [`Button`] components before this set!
    ///
    /// Sets up any [`Button`] components added.
    AddButtonBundle,
    /// apply_deferred
    ApplyDeferredB,
    /// Sends user events from the various [`Button`] components.
    SendButtonEvents,
}

pub fn register_button<T: ButtonEvent>(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_add_button::<T>.in_set(ButtonUiSystemSet::AddButtonBundle),
            on_interact_button::<T>.in_set(ButtonUiSystemSet::SendButtonEvents),
        ),
    )
    .add_event::<T>();
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ButtonUiSystemSet::ApplyDeferredA,
            ButtonUiSystemSet::AddButtonBundle,
            ButtonUiSystemSet::ApplyDeferredB,
            ButtonUiSystemSet::SendButtonEvents,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            apply_deferred.in_set(ButtonUiSystemSet::ApplyDeferredA),
            apply_deferred.in_set(ButtonUiSystemSet::ApplyDeferredB),
        ),
    );
}
