use bevy::{
    a11y::Focus,
    app::Update,
    color::{Color, Srgba},
    core::Name,
    log::{error, info},
    prelude::{
        App, BuildChildren, Commands, Component, Entity, EventReader, IntoSystemConfigs, NodeBundle, OnEnter, Query, Res, ResMut,
        TextBundle, Visibility, With,
    },
    text::{BreakLineOn, Text, TextStyle},
    ui::{AlignItems, BackgroundColor, FlexDirection, Style, Val},
};
use cosmos_core::{
    chat::{ClientSendChatMessageEvent, ServerSendChatMessageEvent},
    netty::{
        sync::events::client_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{
        components::{
            scollable_container::{ScrollBox, ScrollBundle, ScrollerStyles},
            show_cursor::ShowCursor,
            text_input::{InputValue, TextInput, TextInputBundle},
        },
        font::DefaultFont,
        CloseMethod, OpenMenu,
    },
};

#[derive(Component)]
struct ChatContainer;

#[derive(Component)]
struct ReceivedMessagesContainer;

#[derive(Component)]
struct ChatScrollContainer;

fn setup_chat_box(mut commands: Commands, default_font: Res<DefaultFont>) {
    commands
        .spawn((
            ChatContainer,
            OpenMenu::with_close_method(0, CloseMethod::Visibility),
            Name::new("Chat Container"),
            NodeBundle {
                style: Style {
                    top: Val::Percent(20.0),
                    width: Val::Percent(45.0),
                    height: Val::Percent(60.0),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                visibility: Visibility::Hidden,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("Received Messages"),
                ChatScrollContainer,
                ScrollBundle {
                    node_bundle: NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    slider: ScrollBox {
                        styles: ScrollerStyles {
                            scrollbar_color: Srgba {
                                red: 1.0,
                                green: 1.0,
                                blue: 1.0,
                                alpha: 0.4,
                            }
                            .into(),
                            scrollbar_background_color: Color::NONE,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Padding"),
                    NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ));
            })
            .with_children(|p| {
                p.spawn((
                    ReceivedMessagesContainer,
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ));
            });

            p.spawn((
                Name::new("Sending Messages"),
                SendingChatMessageBox,
                TextInputBundle {
                    text_input: TextInput {
                        style: TextStyle {
                            font_size: 24.0,
                            font: default_font.0.clone_weak(),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    node_bundle: NodeBundle {
                        background_color: BackgroundColor(
                            Srgba {
                                red: 0.0,
                                green: 0.0,
                                blue: 0.0,
                                alpha: 0.7,
                            }
                            .into(),
                        ),
                        style: Style {
                            width: Val::Percent(100.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));
        });
}

#[derive(Component)]
struct SendingChatMessageBox;

#[derive(Component)]
struct ChatMessage(f32);

fn display_messages(
    default_font: Res<DefaultFont>,
    mut nevr_chat_msg: EventReader<NettyEventReceived<ServerSendChatMessageEvent>>,
    q_chat_box: Query<Entity, With<ReceivedMessagesContainer>>,
    mut commands: Commands,
) {
    for ev in nevr_chat_msg.read() {
        let msg = &ev.message;

        let text_style = TextStyle {
            font: default_font.0.clone_weak(),
            color: Color::WHITE,
            font_size: 24.0,
        };

        let Ok(parent_ent) = q_chat_box.get_single() else {
            error!("No chat box?");
            return;
        };

        let mut text = Text::from_section(msg, text_style);
        text.linebreak_behavior = BreakLineOn::AnyCharacter;

        commands
            .spawn((
                Name::new("Received chat message"),
                TextBundle {
                    text,
                    ..Default::default()
                },
            ))
            .set_parent(parent_ent);
    }
}

/// # Must be run before [`toggle_chat_box`] or the message will get cleared before this has access
/// to it
fn send_chat_msg(
    inputs: InputChecker,
    mut q_value: Query<&mut InputValue, With<SendingChatMessageBox>>,
    q_chat_box: Query<&Visibility, With<ChatContainer>>,
    mut nevw: NettyEventWriter<ClientSendChatMessageEvent>,
) {
    if !inputs.check_just_pressed(CosmosInputs::SendChatMessage) {
        return;
    }

    if q_chat_box.get_single().map(|x| *x == Visibility::Hidden).unwrap_or(true) {
        return;
    }

    let Ok(mut val) = q_value.get_single_mut() else {
        return;
    };

    let value = val.value();
    if value.is_empty() {
        return;
    }

    nevw.send(ClientSendChatMessageEvent::Global(value.to_owned()));

    // Set val to "" in case toggle chat box and send message are bound to different keys
    val.set_value("");
}

fn toggle_chat_box(
    mut q_input_value: Query<(Entity, &mut InputValue), With<SendingChatMessageBox>>,
    mut q_chat_box: Query<(Entity, &mut Visibility), With<ChatContainer>>,
    mut q_scroll_box: Query<&mut ScrollBox, With<ChatScrollContainer>>,
    inputs: InputChecker,
    mut commands: Commands,
    mut focus: ResMut<Focus>,
) {
    if inputs.check_just_pressed(CosmosInputs::ToggleChat) {
        let Ok((chat_box_ent, mut cb)) = q_chat_box.get_single_mut() else {
            return;
        };

        let Ok((input_ent, mut input_value)) = q_input_value.get_single_mut() else {
            return;
        };
        input_value.set_value("");

        *cb = if *cb == Visibility::Hidden {
            commands.entity(chat_box_ent).insert(ShowCursor);
            focus.0 = Some(input_ent);
            if let Ok(mut scrollbox) = q_scroll_box.get_single_mut() {
                // Start them at the bottom of the chat messages
                scrollbox.scroll_amount = Val::Percent(100.0);
            }
            Visibility::Inherited
        } else {
            commands.entity(chat_box_ent).remove::<ShowCursor>();
            focus.0 = None;
            Visibility::Hidden
        };
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), setup_chat_box);

    app.add_systems(
        Update,
        (display_messages, send_chat_msg, toggle_chat_box)
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    );
}
