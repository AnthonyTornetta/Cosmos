//! A modal where the user enters text

use crate::ui::{
    components::{
        button::{ButtonMessage, CosmosButton},
        modal::{Modal, ModalBody},
        text_input::{InputType, InputValue, TextInput},
    },
    font::DefaultFont,
};

use bevy::{color::palettes::css, input_focus::InputFocus, prelude::*};
use cosmos_core::ecs::NeedsDespawned;

#[derive(Default)]
/// The types of buttons a [`TextModal`] can have.
pub enum TextModalButtons {
    #[default]
    /// Ok + Cancel buttons
    OkCancel,
}

#[derive(Component, Default)]
#[require(Modal)]
/// A modal where the user enters text
pub struct TextModal {
    /// The prompt
    pub prompt: String,
    /// The starting value of that text
    pub starting_value: String,
    /// The buttons this will have
    pub buttons: TextModalButtons,
    /// What type of input we want
    pub input_type: InputType,
}

#[derive(Component)]
struct ModalEntity(Entity);

fn on_add_text_modal(
    q_text_modal: Query<(Entity, &TextModal, &ModalBody), Or<(Added<TextModal>, Added<ModalBody>)>>,
    mut commands: Commands,
    font: Res<DefaultFont>,
    mut focus: ResMut<InputFocus>,
) {
    for (modal_ent, modal, modal_body) in q_text_modal.iter() {
        commands.entity(modal_body.0).with_children(|p| {
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                width: Val::Percent(100.0),
                ..Default::default()
            })
            .with_children(|p| {
                let mut ent = None;
                p.spawn(Node {
                    flex_grow: 1.0,
                    margin: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                })
                .with_children(|p| {
                    if !modal.prompt.is_empty() {
                        p.spawn((
                            Text::new(modal.prompt.clone()),
                            TextFont {
                                font: font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                            Node {
                                margin: UiRect::bottom(Val::Px(10.0)),
                                ..Default::default()
                            },
                        ));
                    }

                    ent = Some(
                        p.spawn((
                            ModalEntity(modal_ent),
                            TextInput {
                                input_type: modal.input_type,
                                ..Default::default()
                            },
                            InputValue::new(modal.starting_value.clone()),
                            TextFont {
                                font: font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                            Node {
                                width: Val::Percent(100.0),
                                ..Default::default()
                            },
                        ))
                        .id(),
                    );
                });

                let ent = ent.expect("Set above");

                focus.set(ent);

                p.spawn(Node {
                    width: Val::Percent(100.0),
                    ..Default::default()
                })
                .with_children(|p| match modal.buttons {
                    TextModalButtons::OkCancel => {
                        p.spawn((
                            TextValueEnt(ent),
                            CosmosButton {
                                text: Some((
                                    "Cancel".into(),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.get(),
                                        ..Default::default()
                                    },
                                    Default::default(),
                                )),
                                ..Default::default()
                            },
                            Node {
                                flex_grow: 1.0,
                                padding: UiRect::all(Val::Px(8.0)),
                                ..Default::default()
                            },
                            BackgroundColor(css::DARK_GREY.into()),
                        ))
                        .observe(
                            |ev: Trigger<ButtonMessage>, q_modal_ent: Query<&ModalEntity>, mut commands: Commands| {
                                let modal_ent = q_modal_ent.get(ev.0).expect("Missing modal entity?");
                                commands.entity(modal_ent.0).insert(NeedsDespawned);
                            },
                        );

                        p.spawn((
                            Node {
                                flex_grow: 1.0,
                                padding: UiRect::all(Val::Px(8.0)),
                                ..Default::default()
                            },
                            TextValueEnt(ent),
                            CosmosButton {
                                text: Some((
                                    "Ok".into(),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.get(),
                                        ..Default::default()
                                    },
                                    TextColor(css::BLACK.into()),
                                )),
                                ..Default::default()
                            },
                            BackgroundColor(css::AQUA.into()),
                        ))
                        .observe(
                            |ev: Trigger<ButtonMessage>,
                             q_text_ent: Query<&TextValueEnt>,
                             q_modal_value: Query<(&InputValue, &ModalEntity)>,
                             mut commands: Commands| {
                                let Ok(tv) = q_text_ent.get(ev.0) else {
                                    return;
                                };
                                let ent = tv.0;
                                let (text, modal_ent) = q_modal_value.get(ent).expect("Missing input?");
                                commands.entity(ent).trigger(TextModalComplete {
                                    text: text.value().to_owned(),
                                });

                                commands.entity(modal_ent.0).insert(NeedsDespawned);
                            },
                        );
                    }
                });
            });
        });
    }
}

#[derive(Component)]
struct TextValueEnt(Entity);

#[derive(Message, Debug)]
#[event(traversal = &'static ChildOf, auto_propagate)]
/// Sent whenever a text modal has its `ok` button pressed
pub struct TextModalComplete {
    /// The value of text the user input
    pub text: String,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_text_modal);
}
