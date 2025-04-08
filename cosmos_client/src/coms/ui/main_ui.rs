use crate::{
    asset::asset_loader::load_assets,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{
        components::{
            button::{register_button, ButtonEvent, CosmosButton},
            scollable_container::ScrollBox,
            show_cursor::ShowCursor,
            text_input::{InputType, InputValue, TextInput},
        },
        font::DefaultFont,
        CloseMenuEvent, CloseMethod, OpenMenu,
    },
};
use bevy::{a11y::Focus, color::palettes::css, prelude::*};
use cosmos_core::structure::ship::pilot::Pilot;
use cosmos_core::{coms::events::SendComsMessageType, state::GameState};
use cosmos_core::{coms::ComsChannelType, netty::system_sets::NetworkingSystemsSet};
use cosmos_core::{coms::ComsMessage, netty::client::LocalPlayer};
use cosmos_core::{
    coms::{events::SendComsMessage, ComsChannel},
    netty::sync::events::client_event::NettyEventWriter,
};

#[derive(Component)]
struct ComsUi;

#[derive(Component)]
struct UiComsMessage;

#[derive(Component)]
struct SelectedComs(Entity);
//
// fn on_add_coms(
//     q_added_coms: Query<(Entity, &Parent, &ComsChannel), Added<ComsChannel>>,
//     q_pilot: Query<&Pilot>,
//     q_local_player: Query<Entity, With<LocalPlayer>>,
//     q_coms_ui: Query<Entity, With<ComsUi>>,
//     mut commands: Commands,
//     coms_assets: Res<ComsAssets>,
//     font: Res<DefaultFont>,
//     mut q_header: Query<(&mut SelectedComs, &mut Text)>,
//     mut q_body: Query<Entity, With<MessageArea>>,
// ) {
//     let Ok((coms_ent, parent, coms)) = q_added_coms.iter().next() else {
//         return;
//     };
//
//     let Ok(pilot) = q_pilot.get(parent.get()) else {
//         return;
//     };
//
//     let Ok(local_player) = q_local_player.get(pilot.entity) else {
//         return;
//     };
//
//     if pilot.entity != local_player {
//         return;
//     }
//
//     let messages = coms.messages.iter().map(|x| x.text.as_str()).collect::<Vec<_>>();
//     if !q_coms_ui.is_empty() {
//         commands
//             .entity(q_body.get_single().expect("Body missing"))
//             .despawn_descendants()
//             .with_children(|p| create_messages_ui(&font, p, &messages));
//
//         let (mut coms, mut text) = q_header.get_single_mut().expect("Header missing");
//         coms.0 = coms_ent;
//         text.0 = "[Ship Name]".into();
//     } else {
//         create_coms_ui(&mut commands, &coms_assets, &font, coms_ent, &messages);
//     }

// }

fn on_change_selected_coms(
    q_changed_coms: Query<(Entity, &Parent, &ComsChannel)>,
    q_pilot: Query<&Pilot>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_coms_ui: Query<Entity, With<ComsUi>>,
    mut commands: Commands,
    coms_assets: Res<ComsAssets>,
    font: Res<DefaultFont>,
    mut q_header: Query<(&SelectedComs, &mut Text), Changed<SelectedComs>>,
    q_body: Query<Entity, With<MessageArea>>,
) {
    let Ok((selected_coms, mut text)) = q_header.get_single_mut() else {
        return;
    };

    let Ok((coms_ent, parent, coms)) = q_changed_coms.get(selected_coms.0) else {
        return;
    };

    let your_ship = parent.get();
    let Ok(pilot) = q_pilot.get(your_ship) else {
        return;
    };

    let Ok(local_player) = q_local_player.get(pilot.entity) else {
        return;
    };

    if pilot.entity != local_player {
        return;
    }

    let messages = coms.messages.iter().collect::<Vec<_>>();
    if !q_coms_ui.is_empty() {
        commands
            .entity(q_body.get_single().expect("Body missing"))
            .despawn_descendants()
            .with_children(|p| create_messages_ui(&font, p, &messages, your_ship));

        text.0 = "[Ship Name]".into();
    } else {
        create_coms_ui(
            &mut commands,
            &coms_assets,
            &font,
            coms_ent,
            &coms.channel_type,
            &messages,
            your_ship,
        );
    }
}

fn on_change_coms(
    q_changed_coms: Query<(Entity, &Parent, &ComsChannel), Or<(Changed<ComsChannel>, Changed<Parent>)>>,
    q_pilot: Query<&Pilot>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_coms_ui: Query<Entity, With<ComsUi>>,
    mut commands: Commands,
    coms_assets: Res<ComsAssets>,
    font: Res<DefaultFont>,
    mut q_header: Query<(&mut SelectedComs, &mut Text)>,
    q_body: Query<Entity, With<MessageArea>>,
) {
    for (coms_ent, parent, coms) in q_changed_coms.iter() {
        let Ok(pilot) = q_pilot.get(parent.get()) else {
            continue;
        };

        let your_ship = pilot.entity;

        if !q_local_player.contains(your_ship) {
            continue;
        }

        // Write code to assert that 1=1 using assert_eq

        let messages = coms.messages.iter().collect::<Vec<_>>();
        if !q_coms_ui.is_empty() {
            let (mut coms, mut text) = q_header.get_single_mut().expect("Header missing");
            if coms.0 != coms_ent {
                continue;
            }

            commands
                .entity(q_body.get_single().expect("Body missing"))
                .despawn_descendants()
                .with_children(|p| create_messages_ui(&font, p, &messages, your_ship));

            coms.0 = coms_ent;
            text.0 = "[Ship Name]".into();
        } else {
            create_coms_ui(
                &mut commands,
                &coms_assets,
                &font,
                coms_ent,
                &coms.channel_type,
                &messages,
                your_ship,
            );
        }
    }
}

fn on_remove_coms(
    mut removed_components: RemovedComponents<ComsChannel>,
    mut q_selected_coms: Query<&mut SelectedComs>,
    q_coms_ui: Query<Entity, With<ComsUi>>,
    q_coms: Query<(Entity, &Parent, &ComsChannel)>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    mut commands: Commands,
) {
    for ent in removed_components.read() {
        let Ok(mut selected_coms) = q_selected_coms.get_single_mut() else {
            continue;
        };

        if selected_coms.0 != ent {
            continue;
        }

        let lp = q_local_player.get_single().expect("Local player missing");

        let mut all_coms = q_coms.iter().filter(|(_, parent, _)| parent.get() == lp);

        if let Some((coms_ent, _, _)) = all_coms.next() {
            selected_coms.0 = coms_ent;
        } else if let Ok(coms_ui_ent) = q_coms_ui.get_single() {
            commands.entity(coms_ui_ent).despawn_recursive();
        }
    }
}

fn create_coms_ui(
    commands: &mut Commands,
    coms_assets: &ComsAssets,
    font: &DefaultFont,
    current_coms_ent: Entity,
    coms_type: &ComsChannelType,
    messages: &[&ComsMessage],
    your_ship: Entity,
) {
    let accent: Color = css::AQUA.into();
    let main_transparent: Color = Srgba::hex("#555555DE").unwrap().into();

    let title_font = TextFont {
        font: font.0.clone_weak(),
        font_size: 24.0,
        ..Default::default()
    };

    let message_font = TextFont {
        font: font.0.clone_weak(),
        font_size: 20.0,
        ..Default::default()
    };

    commands
        .spawn((
            Name::new("Coms Ui"),
            ComsUi,
            OpenMenu::with_close_method(0, CloseMethod::Custom),
            ShowCursor,
            Node {
                margin: UiRect::new(Val::Auto, Val::Px(0.0), Val::Auto, Val::Px(0.0)),
                height: Val::Percent(85.0),
                width: Val::Px(450.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("Coms Header"),
                Node {
                    margin: UiRect::left(Val::Px(50.0)),
                    height: Val::Px(40.0),
                    flex_direction: FlexDirection::Row,
                    max_width: Val::Px(400.0),
                    ..Default::default()
                },
                BorderRadius {
                    top_left: Val::Px(5.0),
                    ..Default::default()
                },
                BackgroundColor(Srgba::hex("#232323").unwrap().into()),
            ))
            .with_children(|p| {
                let btn_node = Node {
                    width: Val::Px(30.0),
                    ..Default::default()
                };

                p.spawn((
                    Name::new("Left btn"),
                    BorderRadius {
                        top_left: Val::Px(5.0),
                        ..Default::default()
                    },
                    BackgroundColor(accent),
                    CosmosButton::<LeftClicked> {
                        text: Some(("<".into(), title_font.clone(), Default::default())),
                        ..Default::default()
                    },
                    btn_node.clone(),
                ));

                p.spawn((
                    Text::new("Cool Ship"),
                    SelectedComs(current_coms_ent),
                    title_font.clone(),
                    TextLayout {
                        justify: JustifyText::Center,
                        ..Default::default()
                    },
                    Node {
                        flex_grow: 1.0,
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                ));

                p.spawn((
                    Name::new("Right btn"),
                    BackgroundColor(accent),
                    CosmosButton::<RightClicked> {
                        text: Some((">".into(), title_font.clone(), Default::default())),
                        ..Default::default()
                    },
                    btn_node,
                ));
            });

            p.spawn((
                Name::new("Main Content"),
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Expand Button"),
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(100.0),
                        ..Default::default()
                    },
                    BorderRadius {
                        top_left: Val::Px(5.0),
                        bottom_left: Val::Px(5.0),
                        ..Default::default()
                    },
                    CosmosButton::<ToggleButton> {
                        image: Some(ImageNode::new(coms_assets.close.clone_weak())),
                        ..Default::default()
                    },
                    BackgroundColor(accent),
                ));
                p.spawn((
                    Name::new("Body"),
                    Node {
                        flex_grow: 1.0,
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(2.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    BackgroundColor(main_transparent),
                    BorderColor(accent),
                ))
                .with_children(|p| {
                    p.spawn((
                        Name::new("Messages ScrollBox"),
                        ScrollBox { ..Default::default() },
                        Node {
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Name::new("Messages"),
                            MessageArea,
                            Node {
                                flex_grow: 1.0,
                                flex_direction: FlexDirection::Column,
                                ..Default::default()
                            },
                        ))
                        .with_children(|p| {
                            create_messages_ui(&message_font.font, p, messages, your_ship);
                        });
                    });

                    p.spawn((
                        Name::new("Send Message Area"),
                        Node {
                            height: Val::Px(200.0),
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        BackgroundColor(Srgba::hex("#444").unwrap().into()),
                        SendMessageArea,
                    ))
                    .with_children(|p| match coms_type {
                        ComsChannelType::Player => {
                            p.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    margin: UiRect::all(Val::Px(10.0)),
                                    ..Default::default()
                                },
                                TextLayout {
                                    linebreak: LineBreak::WordOrCharacter,
                                    ..Default::default()
                                },
                                UiComsMessage,
                                message_font.clone(),
                                TextInput {
                                    input_type: InputType::Text { max_length: Some(100) },
                                    text_node: Node::default(),
                                    ..Default::default()
                                },
                            ));

                            p.spawn((
                                Node {
                                    height: Val::Px(50.0),
                                    width: Val::Percent(100.0),
                                    ..Default::default()
                                },
                                CosmosButton::<SendClicked> {
                                    text: Some(("SEND".into(), message_font.clone(), Default::default())),
                                    ..Default::default()
                                },
                            ));
                        }
                        ComsChannelType::Ai(_) => {
                            p.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    ..Default::default()
                                },
                                CosmosButton::<YesClicked> {
                                    text: Some(("YES".into(), message_font.clone(), Default::default())),
                                    ..Default::default()
                                },
                            ));

                            p.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    ..Default::default()
                                },
                                CosmosButton::<NoClicked> {
                                    text: Some(("NO".into(), message_font.clone(), Default::default())),
                                    ..Default::default()
                                },
                            ));
                        }
                    });
                });
            });
        });
}

fn create_messages_ui(font: &Handle<Font>, messages_container: &mut ChildBuilder, messages: &[&ComsMessage], your_ship: Entity) {
    let message_font = TextFont {
        font: font.clone_weak(),
        font_size: 20.0,
        ..Default::default()
    };

    let accent = css::AQUA.into();

    let you_bg = BackgroundColor(
        Srgba {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 0.3,
        }
        .into(),
    );

    let other_bg = BackgroundColor(Color::NONE);

    if let Some(first) = messages.first() {
        messages_container.spawn((
            Name::new("Message"),
            Text::new(&first.text),
            Node {
                margin: UiRect::new(
                    Val::Px(10.0),
                    Val::Px(10.0),
                    Val::Px(10.0),
                    Val::Px(
                        messages
                            .get(1)
                            .map(|x| if x.sender != first.sender { 10.0 } else { 0.0 })
                            .unwrap_or(0.0),
                    ),
                ),
                ..Default::default()
            },
            message_font.clone(),
            BorderColor(accent),
            if first.sender == your_ship { you_bg } else { other_bg },
        ));
    }

    if messages.len() < 2 {
        // If 1 or 0 messages, the rest of the function is unnecessary and cause double-renders.
        return;
    }

    for [above, msg, below] in messages.array_windows::<3>() {
        let top = if above.sender == msg.sender { Val::Px(0.0) } else { Val::Px(10.0) };

        let bottom = if below.sender == msg.sender { Val::Px(0.0) } else { Val::Px(10.0) };

        messages_container.spawn((
            Name::new("Message"),
            Text::new(&msg.text),
            Node {
                margin: UiRect::new(Val::Px(10.0), Val::Px(10.0), top, bottom),
                ..Default::default()
            },
            message_font.clone(),
            BorderColor(accent),
            if msg.sender == your_ship { you_bg } else { other_bg },
        ));
    }

    if let Some([prev, last]) = messages.array_windows::<2>().last() {
        messages_container.spawn((
            Name::new("Message"),
            Text::new(&last.text),
            Node {
                margin: UiRect::new(
                    Val::Px(10.0),
                    Val::Px(10.0),
                    Val::Px(if prev.sender != last.sender { 10.0 } else { 0.0 }),
                    Val::Px(10.0),
                ),
                ..Default::default()
            },
            message_font.clone(),
            BorderColor(accent),
            if last.sender == your_ship { you_bg } else { other_bg },
        ));
    }
}

#[derive(Component)]
struct MessageArea;

#[derive(Component)]
struct SendMessageArea;

#[derive(Resource, Debug)]
pub struct ComsAssets {
    open: Handle<Image>,
    close: Handle<Image>,
}

#[derive(Event, Debug)]
struct LeftClicked;

impl ButtonEvent for LeftClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}
#[derive(Event, Debug)]
struct RightClicked;

impl ButtonEvent for RightClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct SendClicked;

impl ButtonEvent for SendClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct NoClicked;

impl ButtonEvent for NoClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct YesClicked;

impl ButtonEvent for YesClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct ToggleButton;

impl ButtonEvent for ToggleButton {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn on_toggle(
    mut commands: Commands,
    inputs: InputChecker,
    mut q_coms_ui: Query<(Entity, &mut Node, Has<ShowCursor>), With<ComsUi>>,
    mut evr_toggle: EventReader<ToggleButton>,
    coms_assets: Res<ComsAssets>,
    mut q_toggle_button: Query<&mut CosmosButton<ToggleButton>>,
    mut focused: ResMut<Focus>,
) {
    if evr_toggle.read().next().is_none() & !inputs.check_just_pressed(CosmosInputs::ToggleComs) {
        return;
    }

    for (entity, mut node, has) in q_coms_ui.iter_mut() {
        if has {
            minimize_ui(&mut commands, &coms_assets, &mut q_toggle_button, entity, &mut node, &mut focused);
        } else {
            node.right = Val::Px(0.0);
            commands
                .entity(entity)
                .insert((ShowCursor, OpenMenu::with_close_method(0, CloseMethod::Custom)));

            if let Ok(mut tb) = q_toggle_button.get_single_mut() {
                tb.image = Some(ImageNode::new(coms_assets.close.clone_weak()));
            }
        }
    }
}

fn minimize_ui(
    commands: &mut Commands,
    coms_assets: &ComsAssets,
    q_toggle_button: &mut Query<&mut CosmosButton<ToggleButton>>,
    entity: Entity,
    node: &mut Node,
    focused: &mut Focus,
) {
    let Val::Px(w) = node.width else {
        return;
    };

    node.right = Val::Px(-w + 50.0);
    commands.entity(entity).remove::<ShowCursor>().remove::<OpenMenu>();
    focused.0 = None;
    if let Ok(mut tb) = q_toggle_button.get_single_mut() {
        tb.image = Some(ImageNode::new(coms_assets.open.clone_weak()));
    }
}

fn on_close_menu(
    coms_assets: Res<ComsAssets>,
    mut evr: EventReader<CloseMenuEvent>,
    mut q_coms_ui: Query<&mut Node, With<ComsUi>>,
    mut commands: Commands,
    mut q_toggle_button: Query<&mut CosmosButton<ToggleButton>>,
    mut focused: ResMut<Focus>,
) {
    for ev in evr.read() {
        if let Ok(mut node) = q_coms_ui.get_mut(ev.0) {
            minimize_ui(&mut commands, &coms_assets, &mut q_toggle_button, ev.0, &mut node, &mut focused);
        }
    }
}

fn send_text(
    mut nevw_send_coms_message: NettyEventWriter<SendComsMessage>,
    q_selected_coms: Query<&SelectedComs>,
    mut q_text_value: Query<&mut InputValue, With<UiComsMessage>>,
    mut evr_send: EventReader<SendClicked>,
    inputs: InputChecker,
    q_coms_channel: Query<&ComsChannel>,
) {
    if evr_send.read().next().is_none() & !inputs.check_just_pressed(CosmosInputs::SendComs) {
        return;
    }

    let Ok(mut text) = q_text_value.get_single_mut() else {
        return;
    };

    let Ok(selected) = q_selected_coms.get_single() else {
        return;
    };

    let val = text.value();

    if val.is_empty() {
        return;
    }

    let Ok(coms_channel) = q_coms_channel.get(selected.0) else {
        return;
    };

    info!("Sending message: {}", val);
    nevw_send_coms_message.send(SendComsMessage {
        message: SendComsMessageType::Message(val.to_owned()),
        to: coms_channel.with,
    });

    *text = Default::default();
}

fn yes_clicked(
    mut nevw_send_coms_message: NettyEventWriter<SendComsMessage>,
    q_selected_coms: Query<&SelectedComs>,
    mut evr_send: EventReader<YesClicked>,
    q_coms_channel: Query<&ComsChannel>,
) {
    if evr_send.read().next().is_none() {
        return;
    }

    let Ok(selected) = q_selected_coms.get_single() else {
        return;
    };

    let Ok(coms_channel) = q_coms_channel.get(selected.0) else {
        return;
    };

    info!("Sending yes");
    nevw_send_coms_message.send(SendComsMessage {
        message: SendComsMessageType::Yes,
        to: coms_channel.with,
    });
}

fn no_clicked(
    mut nevw_send_coms_message: NettyEventWriter<SendComsMessage>,
    q_selected_coms: Query<&SelectedComs>,
    mut evr_send: EventReader<NoClicked>,
    q_coms_channel: Query<&ComsChannel>,
) {
    if evr_send.read().next().is_none() {
        return;
    }

    let Ok(selected) = q_selected_coms.get_single() else {
        return;
    };

    let Ok(coms_channel) = q_coms_channel.get(selected.0) else {
        return;
    };

    info!("Sending yes");
    nevw_send_coms_message.send(SendComsMessage {
        message: SendComsMessageType::No,
        to: coms_channel.with,
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_remove_coms,
            on_change_coms,
            on_change_selected_coms,
            on_toggle,
            on_close_menu,
            send_text,
            yes_clicked,
            no_clicked,
        )
            .chain()
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between),
    );

    register_button::<LeftClicked>(app);
    register_button::<RightClicked>(app);
    register_button::<SendClicked>(app);
    register_button::<YesClicked>(app);
    register_button::<NoClicked>(app);
    register_button::<ToggleButton>(app);

    load_assets::<Image, ComsAssets, 2>(
        app,
        GameState::Loading,
        ["cosmos/images/ui/open-coms.png", "cosmos/images/ui/close-coms.png"],
        |mut cmds, [(open, _), (close, _)]| cmds.insert_resource(ComsAssets { open, close }),
    );
}
