//! Client-side chat logic

use bevy::{input_focus::InputFocus, prelude::*};
use cosmos_core::{
    chat::{ClientSendChatMessageEvent, ServerSendChatMessageEvent},
    commands::ClientCommandEvent,
    ecs::NeedsDespawned,
    netty::{
        sync::events::client_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{
        CloseMethod, OpenMenu,
        components::{
            scollable_container::{ScrollBox, ScrollerStyles},
            show_cursor::ShowCursor,
            text_input::{InputValue, TextInput},
        },
        font::DefaultFont,
        hide::DontHideOnToggleUi,
        pause::CloseMenusSet,
    },
};

#[derive(Component)]
struct ChatContainer;

#[derive(Component)]
struct ReceivedMessagesContainer;

#[derive(Component)]
struct ChatScrollContainer;

#[derive(Component)]
struct SendingChatMessageBox;

#[derive(Component)]
struct ChatMessage(f32);

#[derive(Component)]
struct ChatDisplayReceivedMessagesContainer;

#[derive(Component)]
struct ChatDisplay;

fn toggle_chat_display_visibility(
    mut q_chat_display: Query<&mut Visibility, (Without<ChatContainer>, With<ChatDisplay>)>,
    q_chat_box: Query<&Visibility, (Changed<Visibility>, With<ChatContainer>)>,
) {
    let Ok(changed_vis) = q_chat_box.single() else {
        return;
    };

    let Ok(mut vis) = q_chat_display.single_mut() else {
        return;
    };

    match *changed_vis {
        Visibility::Hidden => *vis = Visibility::Inherited,
        _ => *vis = Visibility::Hidden,
    };
}

fn setup_chat_display(mut commands: Commands) {
    commands
        .spawn((
            ChatDisplay,
            Name::new("Chat Display"),
            Node {
                top: Val::Percent(20.0),
                width: Val::Percent(45.0),
                height: Val::Percent(60.0),
                overflow: Overflow {
                    x: OverflowAxis::Hidden,
                    y: OverflowAxis::Hidden,
                },
                flex_direction: FlexDirection::ColumnReverse,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                ChatDisplayReceivedMessagesContainer,
                Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ));
        });
}

const CHAT_MSG_ALIVE_SEC: f32 = 10.0;

fn fade_chat_messages(
    q_time: Res<Time>,
    mut writer: TextUiWriter,
    mut q_chat_msg: Query<(Entity, &mut ChatMessage)>,
    mut commands: Commands,
) {
    let delta = q_time.delta_secs();
    for (ent, mut chat_msg) in q_chat_msg.iter_mut() {
        chat_msg.0 -= delta;

        if chat_msg.0 <= 0.0 {
            commands.entity(ent).insert(NeedsDespawned);
        } else {
            writer.for_each_color(ent, |mut c| c.set_alpha(chat_msg.0 / CHAT_MSG_ALIVE_SEC))
        }
    }
}

fn setup_chat_box(mut commands: Commands, default_font: Res<DefaultFont>) {
    commands
        .spawn((
            ChatContainer,
            DontHideOnToggleUi,
            Name::new("Chat Container"),
            Node {
                top: Val::Percent(20.0),
                width: Val::Percent(45.0),
                height: Val::Percent(60.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("Received Messages"),
                ChatScrollContainer,
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ScrollBox {
                    styles: ScrollerStyles {
                        scrollbar_color: Srgba {
                            red: 1.0,
                            green: 1.0,
                            blue: 1.0,
                            alpha: 0.4,
                        }
                        .into(),
                        scrollbar_background_color: Color::NONE,
                    },
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Padding"),
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                ));
            })
            .with_children(|p| {
                p.spawn((
                    ReceivedMessagesContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                ));
            });

            p.spawn((
                Name::new("Sending Messages"),
                SendingChatMessageBox,
                TextFont {
                    font_size: 24.0,
                    font: default_font.0.clone(),
                    ..Default::default()
                },
                BackgroundColor(
                    Srgba {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 0.7,
                    }
                    .into(),
                ),
                Node {
                    width: Val::Percent(100.0),
                    ..Default::default()
                },
                TextInput { ..Default::default() },
            ));
        });
}

fn display_messages(
    default_font: Res<DefaultFont>,
    mut nevr_chat_msg: EventReader<NettyEventReceived<ServerSendChatMessageEvent>>,
    q_chat_box: Query<Entity, With<ReceivedMessagesContainer>>,
    q_display_box: Query<Entity, With<ChatDisplayReceivedMessagesContainer>>,
    mut commands: Commands,
) {
    for ev in nevr_chat_msg.read() {
        let msg = &ev.message;

        let text_style = TextFont {
            font: default_font.0.clone_weak(),
            font_size: 24.0,
            ..Default::default()
        };

        let Ok(chat_box) = q_chat_box.single() else {
            error!("No chat box?");
            return;
        };

        let Ok(display_box) = q_display_box.single() else {
            error!("No display box?");
            return;
        };

        let text = Text::new(msg);
        let text_layout = TextLayout {
            linebreak: LineBreak::AnyCharacter,
            ..Default::default()
        };

        commands.spawn((
            Name::new("Received chat message"),
            text.clone(),
            text_style.clone(),
            text_layout,
            ChildOf(chat_box),
        ));

        commands.spawn((
            Name::new("Display received chat message"),
            ChatMessage(CHAT_MSG_ALIVE_SEC),
            text,
            text_style.clone(),
            text_layout,
            ChildOf(display_box),
        ));
    }
}

/// # Must be run before [`toggle_chat_box`] or the message will get cleared before this has access
/// to it
fn send_chat_msg(
    inputs: InputChecker,
    mut q_value: Query<&mut InputValue, With<SendingChatMessageBox>>,
    q_chat_box: Query<&Visibility, With<ChatContainer>>,
    mut nevw_chat: NettyEventWriter<ClientSendChatMessageEvent>,
    mut nevw_command: NettyEventWriter<ClientCommandEvent>,
) {
    if !inputs.check_just_pressed(CosmosInputs::SendChatMessage) {
        return;
    }

    if q_chat_box.single().map(|x| *x == Visibility::Hidden).unwrap_or(true) {
        return;
    }

    let Ok(mut val) = q_value.single_mut() else {
        return;
    };

    let value = val.value();
    if value.is_empty() {
        return;
    }

    if let Some(stripped) = value.strip_prefix("/") {
        nevw_command.write(ClientCommandEvent {
            command_text: stripped.to_owned(),
        });
    } else {
        nevw_chat.write(ClientSendChatMessageEvent::Global(value.to_owned()));
    }

    // Set val to "" in case toggle chat box and send message are bound to different keys
    val.set_value("");
}

fn toggle_chat_box(
    mut q_input_value: Query<(Entity, &mut InputValue), With<SendingChatMessageBox>>,
    mut q_chat_box: Query<(Entity, &mut Visibility), With<ChatContainer>>,
    mut q_scroll_box: Query<&mut ScrollBox, With<ChatScrollContainer>>,
    inputs: InputChecker,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
    q_open_menus: Query<(), With<OpenMenu>>,
) {
    if inputs.check_just_pressed(CosmosInputs::ToggleChat) {
        let Ok((chat_box_ent, mut cb)) = q_chat_box.single_mut() else {
            return;
        };

        *cb = if *cb == Visibility::Hidden {
            if !q_open_menus.is_empty() {
                return;
            }

            let Ok((input_ent, mut input_value)) = q_input_value.single_mut() else {
                return;
            };
            input_value.set_value("");

            commands
                .entity(chat_box_ent)
                .insert(ShowCursor)
                .insert(OpenMenu::with_close_method(0, CloseMethod::Visibility));
            focus.0 = Some(input_ent);
            if let Ok(mut scrollbox) = q_scroll_box.single_mut() {
                // Start them at the bottom of the chat messages
                scrollbox.scroll_amount = Val::Percent(100.0);
            }
            Visibility::Inherited
        } else {
            commands.entity(chat_box_ent).remove::<ShowCursor>().remove::<OpenMenu>();
            focus.0 = None;
            Visibility::Hidden
        };
    }
}

/// The maximum number of chat messages that can be stored in the chat box history.
const MAX_MESSAGES: usize = 100;

fn remove_very_old_messages(mut commands: Commands, q_children: Query<&Children, With<ReceivedMessagesContainer>>) {
    let Ok(children) = q_children.single() else {
        return;
    };

    for ent in children.iter().take(children.len().max(MAX_MESSAGES) - MAX_MESSAGES) {
        commands.entity(ent).insert(NeedsDespawned);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), (setup_chat_display, setup_chat_box));

    app.add_systems(
        Update,
        (
            display_messages,
            send_chat_msg,
            toggle_chat_box,
            fade_chat_messages,
            remove_very_old_messages,
            toggle_chat_display_visibility,
        )
            .chain()
            .run_if(in_state(GameState::Playing))
            .after(CloseMenusSet::CloseMenus),
    );
}
