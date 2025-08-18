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
pub enum TextModalButtons {
    #[default]
    YesNo,
}

#[derive(Component, Default)]
#[require(Modal)]
pub struct ConfirmModal {
    pub prompt: String,
    pub buttons: TextModalButtons,
}

#[derive(Component)]
struct ModalEntity(Entity);

fn on_add_text_modal(
    q_text_modal: Query<(Entity, &ConfirmModal, &ModalBody), Or<(Added<ConfirmModal>, Added<ModalBody>)>>,
    mut commands: Commands,
    font: Res<DefaultFont>,
) {
    for (modal_ent, modal, modal_body) in q_text_modal.iter() {
        commands.entity(modal_body.0).with_children(|p| {
            p.spawn(
                (Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                }),
            )
            .with_children(|p| {
                p.spawn((
                    Text::new(modal.prompt.clone()),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                    Node {
                        flex_grow: 1.0,
                        margin: UiRect::all(Val::Px(10.0)),
                        ..Default::default()
                    },
                ));

                p.spawn((Node { ..Default::default() })).with_children(|p| match modal.buttons {
                    TextModalButtons::YesNo => {
                        p.spawn((
                            ModalEntity(modal_ent),
                            CosmosButton::<CancelButton> {
                                text: Some((
                                    "No".into(),
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
                            BackgroundColor(css::AQUA.into()),
                            ModalEntity(modal_ent),
                            CosmosButton::<OkButton> {
                                text: Some((
                                    "Yes".into(),
                                    TextFont {
                                        font_size: 24.0,
                                        font: font.get(),
                                        ..Default::default()
                                    },
                                    TextColor(css::BLACK.into()),
                                )),
                                ..Default::default()
                            },
                            Node {
                                flex_grow: 1.0,
                                padding: UiRect::all(Val::Px(8.0)),
                                ..Default::default()
                            },
                        ));
                    }
                });
            });
        });
    }
}

create_private_button_event!(OkButton);
create_private_button_event!(CancelButton);

#[derive(Event, Debug)]
#[event(traversal = &'static ChildOf, auto_propagate)]
pub struct ConfirmModalComplete {
    pub confirmed: bool,
}

fn on_ok(mut commands: Commands, q_value: Query<&ModalEntity>, mut evr_ok: EventReader<OkButton>) {
    for ev in evr_ok.read() {
        let modal_ent = q_value.get(ev.0).expect("Missing input?");
        commands
            .entity(modal_ent.0)
            .trigger(ConfirmModalComplete { confirmed: true })
            .insert(NeedsDespawned);
    }
}

fn on_cancel(mut commands: Commands, q_value: Query<&ModalEntity>, mut evr_cancel: EventReader<CancelButton>) {
    for ev in evr_cancel.read() {
        let modal_ent = q_value.get(ev.0).expect("Missing modal entity?");
        commands
            .entity(modal_ent.0)
            .trigger(ConfirmModalComplete { confirmed: false })
            .insert(NeedsDespawned);
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<OkButton>(app);
    register_button::<CancelButton>(app);

    app.add_systems(Update, (on_add_text_modal, on_ok, on_cancel));
}
