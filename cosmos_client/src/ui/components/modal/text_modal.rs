//! A modal where the user enters text

use crate::{
    create_private_button_event,
    ui::{
        components::{
            button::{CosmosButton, register_button},
            modal::{Modal, ModalBody},
            text_input::{InputType, InputValue, TextInput},
        },
        font::DefaultFont,
    },
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
                    ..Default::default()
                })
                .with_children(|p| {
                    ent = Some(
                        p.spawn((
                            ModalEntity(modal_ent),
                            TextInput {
                                input_type: modal.input_type,
                                ..Default::default()
                            },
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
                            CosmosButton::<CancelButton> {
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
                        ));

                        p.spawn((
                            Node {
                                flex_grow: 1.0,
                                padding: UiRect::all(Val::Px(8.0)),
                                ..Default::default()
                            },
                            TextValueEnt(ent),
                            CosmosButton::<OkButton> {
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
                        ));
                    }
                });
            });
        });
    }
}

#[derive(Component)]
struct TextValueEnt(Entity);

create_private_button_event!(OkButton);
create_private_button_event!(CancelButton);

#[derive(Event, Debug)]
#[event(traversal = &'static ChildOf, auto_propagate)]
/// Sent whenever a text modal has its `ok` button pressed
pub struct TextModalComplete {
    /// The value of text the user input
    pub text: String,
}

fn on_ok(
    mut commands: Commands,
    q_text_modal_ent: Query<&TextValueEnt>,
    q_value: Query<(&InputValue, &ModalEntity)>,
    mut evr_ok: EventReader<OkButton>,
) {
    for ev in evr_ok.read() {
        let Ok(tv) = q_text_modal_ent.get(ev.0) else {
            continue;
        };
        let ent = tv.0;
        let (text, modal_ent) = q_value.get(ent).expect("Missing input?");
        commands.entity(ent).trigger(TextModalComplete {
            text: text.value().to_owned(),
        });

        commands.entity(modal_ent.0).insert(NeedsDespawned);
    }
}

fn on_cancel(
    mut commands: Commands,
    q_text_modal_ent: Query<&TextValueEnt>,
    q_value: Query<&ModalEntity>,
    mut evr_cancel: EventReader<CancelButton>,
) {
    for ev in evr_cancel.read() {
        let Ok(tv) = q_text_modal_ent.get(ev.0) else {
            continue;
        };
        let ent = tv.0;
        let modal_ent = q_value.get(ent).expect("Missing modal entity?");
        commands.entity(modal_ent.0).insert(NeedsDespawned);
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<OkButton>(app);
    register_button::<CancelButton>(app);

    app.add_systems(Update, (on_add_text_modal, on_ok, on_cancel));
}
