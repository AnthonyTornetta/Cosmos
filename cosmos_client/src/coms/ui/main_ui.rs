use crate::{
    asset::asset_loader::load_assets,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{
        CloseMenuMessage, CloseMethod, OpenMenu, UiSystemSet,
        components::{
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            scollable_container::{ScrollBox, ScrollerStyles},
            show_cursor::ShowCursor,
            text_input::{InputType, InputValue, PlaceholderText, TextInput},
        },
        font::DefaultFont,
    },
};
use bevy::{input_focus::InputFocus, prelude::*};
use cosmos_core::{
    coms::{
        ComsChannel, ComsChannelType, ComsMessage,
        events::{RequestCloseComsMessage, SendComsMessage, SendComsMessageType},
    },
    faction::{Faction, FactionId, FactionRelation, Factions},
    netty::{client::LocalPlayer, sync::events::client_event::NettyMessageWriter},
    physics::location::Location,
    state::GameState,
    structure::ship::pilot::Pilot,
};

use super::style;

const COMS_TAB_WIDTH: f32 = 32.0;
const COMS_WIDTH: f32 = 432.0;

#[derive(Component)]
struct ComsUi;

#[derive(Component)]
struct UiComsMessage;

#[derive(Component)]
struct SelectedComs(Entity);

#[derive(Component)]
struct ResponseArea;

#[derive(Component)]
struct ChannelNavigation;

#[derive(Component)]
struct TransmissionScroll;

#[derive(Component)]
struct ScrollToLatest;

#[derive(Component)]
struct RenderedChannelType(ComsChannelType);

#[derive(Component)]
enum ContactDetail {
    Name,
    Description,
    Relation,
}

/// Refresh only the message area for new messages, preserving an unsent player draft.
fn sync_coms_ui(
    mut commands: Commands,
    q_coms: Query<(Entity, &ChildOf, Ref<ComsChannel>)>,
    q_local_pilot: Query<&Pilot, With<LocalPlayer>>,
    mut q_ui: Query<(Entity, &mut SelectedComs, &mut RenderedChannelType), With<ComsUi>>,
    q_body: Query<Entity, With<MessageArea>>,
    q_scroll: Query<Entity, With<TransmissionScroll>>,
    q_responses: Query<Entity, With<ResponseArea>>,
    mut q_navigation: Query<&mut Node, With<ChannelNavigation>>,
    font: Res<DefaultFont>,
    coms_assets: Res<ComsAssets>,
    mut focused: ResMut<InputFocus>,
) {
    let Ok(pilot) = q_local_pilot.single() else {
        return;
    };
    let mut channels = q_coms
        .iter()
        .filter(|(_, parent, _)| parent.parent() == pilot.entity)
        .collect::<Vec<_>>();
    channels.sort_unstable_by_key(|(entity, _, _)| entity.to_bits());
    for mut node in &mut q_navigation {
        node.display = if channels.len() > 1 { Display::Flex } else { Display::None };
    }
    let Some((first_entity, _, first_channel)) = channels.first() else {
        // The last channel closed. Despawning also releases the menu's cursor claim.
        if let Ok((entity, _, _)) = q_ui.single() {
            commands.entity(entity).despawn();
            focused.0 = None;
        }
        return;
    };
    let Ok((_, mut selected, mut rendered_type)) = q_ui.single_mut() else {
        create_coms_ui(&mut commands, &coms_assets, &font, *first_entity, first_channel, pilot.entity);
        return;
    };
    let (entity, _, channel) = channels.iter().find(|(entity, _, _)| *entity == selected.0).unwrap_or(&channels[0]);
    let selection_changed = selected.is_changed() || selected.0 != *entity;
    if selected.0 != *entity {
        selected.0 = *entity;
    }
    if (selection_changed || channel.is_changed())
        && let Ok(body) = q_body.single()
    {
        commands.entity(body).despawn_related::<Children>().with_children(|p| {
            create_messages_ui(&font.0, p, &channel.messages, pilot.entity);
        });
        for entity in &q_scroll {
            commands.entity(entity).insert(ScrollToLatest);
        }
    }
    if selection_changed || rendered_type.0 != channel.channel_type {
        focused.0 = None;
        rendered_type.0 = channel.channel_type;
        if let Ok(responses) = q_responses.single() {
            commands.entity(responses).despawn_related::<Children>().with_children(|p| {
                create_responses_ui(p, &font.0, channel.channel_type);
            });
        }
    }
}

fn update_contact_details(
    q_selected: Query<&SelectedComs>,
    q_coms: Query<&ComsChannel>,
    q_local_player: Query<(Entity, &Pilot), With<LocalPlayer>>,
    q_names: Query<&Name>,
    q_locations: Query<&Location>,
    q_faction: Query<&FactionId>,
    factions: Res<Factions>,
    mut q_details: Query<(&ContactDetail, &mut Text, &mut TextColor)>,
) {
    let Ok(selected) = q_selected.single() else {
        return;
    };
    let Ok(channel) = q_coms.get(selected.0) else {
        return;
    };
    let Ok((player, pilot)) = q_local_player.single() else {
        return;
    };
    let contact_faction = q_faction.get(channel.with).ok().and_then(|id| factions.from_id(id));
    let player_faction = q_faction.get(player).ok().and_then(|id| factions.from_id(id));
    let relation = Faction::relation_with_option(contact_faction, player_faction);
    let faction_name = contact_faction.map(|f| f.name()).unwrap_or("Independent vessel");
    let description = match (q_locations.get(pilot.entity), q_locations.get(channel.with)) {
        (Ok(ours), Ok(theirs)) => {
            let meters = ours.distance_sqrd(theirs).sqrt();
            if meters >= 1000.0 {
                format!("{faction_name}  /  {:.1} km", meters / 1000.0)
            } else {
                format!("{faction_name}  /  {meters:.0} m")
            }
        }
        _ => faction_name.to_owned(),
    };
    for (detail, mut text, mut color) in &mut q_details {
        let (value, tint) = match detail {
            ContactDetail::Name => (
                q_names
                    .get(channel.with)
                    .map(|name| name.as_str())
                    .unwrap_or("Unknown vessel")
                    .to_owned(),
                style::TEXT,
            ),
            ContactDetail::Description => (description.clone(), style::MUTED),
            ContactDetail::Relation => match relation {
                FactionRelation::Ally => ("ALLY".to_owned(), style::ACCENT),
                FactionRelation::Neutral => ("NEUTRAL".to_owned(), style::MUTED),
                FactionRelation::Enemy => ("HOSTILE".to_owned(), style::DANGER),
            },
        };
        text.set_if_neq(Text::new(value));
        color.set_if_neq(TextColor(tint));
    }
}

fn create_coms_ui(
    commands: &mut Commands,
    coms_assets: &ComsAssets,
    font: &DefaultFont,
    current_coms_ent: Entity,
    channel: &ComsChannel,
    your_ship: Entity,
) {
    commands
        .spawn((
            Name::new("Coms UI"),
            ComsUi,
            SelectedComs(current_coms_ent),
            RenderedChannelType(channel.channel_type),
            OpenMenu::with_close_method(0, CloseMethod::Custom),
            ShowCursor,
            DespawnOnExit(GameState::Playing),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Percent(18.0),
                width: Val::Px(COMS_WIDTH),
                max_width: Val::Vw(96.0),
                max_height: Val::Percent(72.0),
                align_items: AlignItems::Start,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Name::new("Minimize comms"),
                ToggleButton,
                Node {
                    width: Val::Px(COMS_TAB_WIDTH),
                    height: Val::Px(40.0),
                    flex_shrink: 0.0,
                    padding: UiRect::all(Val::Px(5.0)),
                    ..Default::default()
                },
                BackgroundColor(style::SURFACE),
                CosmosButton {
                    image: Some(ImageNode::new(coms_assets.close.clone())),
                    ..Default::default()
                },
            ))
            .observe(on_toggle);
            p.spawn((
                Name::new("Quick hail panel"),
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Px(COMS_WIDTH - COMS_TAB_WIDTH),
                    min_width: Val::Px(0.0),
                    max_height: Val::Vh(72.0),
                    border: UiRect {
                        top: Val::Px(3.0),
                        ..UiRect::all(Val::Px(1.0))
                    },
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    overflow: Overflow::clip(),
                    ..Default::default()
                },
                BackgroundColor(style::BACKGROUND),
                BorderColor {
                    top: style::ACCENT,
                    ..BorderColor::all(style::BORDER)
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Coms channel header"),
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        flex_shrink: 0.0,
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..Default::default()
                    },
                    BorderColor::all(style::BORDER),
                ))
                .with_children(|p| {
                    style::label(p, &font.0, "SHIP COMMS", 12.0, style::MUTED);
                    style::label(p, &font.0, "CONNECTED", 11.0, style::ACCENT);
                });
                p.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::new(Val::Px(18.0), Val::Px(18.0), Val::Px(18.0), Val::Px(12.0)),
                    row_gap: Val::Px(7.0),
                    flex_shrink: 0.0,
                    ..Default::default()
                })
                .with_children(|p| {
                    p.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn((
                            ContactDetail::Name,
                            Text::new("Unknown vessel"),
                            style::font(&font.0, 22.0),
                            TextColor(style::TEXT),
                            Node {
                                flex_grow: 1.0,
                                flex_basis: Val::Px(0.0),
                                min_width: Val::Px(0.0),
                                ..Default::default()
                            },
                        ));
                        p.spawn((
                            ContactDetail::Relation,
                            Text::new("NEUTRAL"),
                            style::font(&font.0, 11.0),
                            TextColor(style::MUTED),
                            BorderColor::all(style::BORDER),
                            Node {
                                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(3.0)),
                                flex_shrink: 0.0,
                                ..Default::default()
                            },
                        ));
                    });
                    p.spawn((
                        ContactDetail::Description,
                        Text::new("Independent vessel"),
                        style::font(&font.0, 12.0),
                        TextColor(style::MUTED),
                    ));
                });
                p.spawn((
                    Name::new("Transmission scroll area"),
                    TransmissionScroll,
                    ScrollToLatest,
                    ScrollBox {
                        scroll_amount: Val::Percent(100.0),
                        styles: ScrollerStyles {
                            scrollbar_color: style::BORDER,
                            scrollbar_background_color: style::SURFACE,
                        },
                        ..Default::default()
                    },
                    Node {
                        height: Val::Px(120.0),
                        min_height: Val::Px(48.0),
                        margin: UiRect::horizontal(Val::Px(18.0)),
                        ..Default::default()
                    },
                    BackgroundColor(style::SURFACE),
                ))
                .with_children(|p| {
                    p.spawn((
                        Name::new("Messages"),
                        MessageArea,
                        Node {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(12.0)),
                            row_gap: Val::Px(12.0),
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| create_messages_ui(&font.0, p, &channel.messages, your_ship));
                });
                p.spawn((
                    ResponseArea,
                    Node {
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(14.0)),
                        row_gap: Val::Px(8.0),
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                ))
                .with_children(|p| create_responses_ui(p, &font.0, channel.channel_type));
                p.spawn((
                    Name::new("Coms footer"),
                    Node {
                        border: UiRect::top(Val::Px(1.0)),
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        flex_shrink: 0.0,
                        ..Default::default()
                    },
                    BorderColor::all(style::BORDER),
                ))
                .with_children(|p| {
                    p.spawn((
                        ChannelNavigation,
                        Node {
                            display: Display::None,
                            column_gap: Val::Px(4.0),
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Node {
                                width: Val::Px(28.0),
                                height: Val::Px(30.0),
                                ..Default::default()
                            },
                            coms_button(&font.0, "<", false),
                        ))
                        .observe(on_left_clicked);
                        p.spawn((
                            Node {
                                width: Val::Px(28.0),
                                height: Val::Px(30.0),
                                ..Default::default()
                            },
                            coms_button(&font.0, ">", false),
                        ))
                        .observe(on_right_clicked);
                    });
                    p.spawn((
                        Node {
                            height: Val::Px(30.0),
                            margin: UiRect::left(Val::Auto),
                            padding: UiRect::horizontal(Val::Px(8.0)),
                            ..Default::default()
                        },
                        coms_button(&font.0, "End transmission", false),
                    ))
                    .observe(end_selected_coms);
                });
            });
        });
}

fn coms_button(font: &Handle<Font>, text: &str, primary: bool) -> CosmosButton {
    CosmosButton {
        text: Some((text.to_owned(), style::font(font, 14.0), TextColor(style::TEXT))),
        button_styles: Some(ButtonStyles {
            background_color: if primary { style::SELECTED } else { style::SURFACE },
            foreground_color: style::TEXT,
            hover_background_color: style::SELECTED,
            hover_foreground_color: style::TEXT,
            press_background_color: style::BORDER,
            press_foreground_color: style::TEXT,
        }),
        ..Default::default()
    }
}

fn create_responses_ui(p: &mut ChildSpawnerCommands, font: &Handle<Font>, channel_type: ComsChannelType) {
    let button_node = Node {
        width: Val::Percent(100.0),
        height: Val::Px(40.0),
        border: UiRect::all(Val::Px(1.0)),
        border_radius: BorderRadius::all(Val::Px(3.0)),
        ..Default::default()
    };
    match channel_type {
        ComsChannelType::Ai(_) => {
            p.spawn((
                Name::new("Yes reply"),
                button_node.clone(),
                BorderColor::all(style::ACCENT),
                coms_button(font, "Yes", true),
            ))
            .observe(yes_clicked);
            p.spawn((
                Name::new("No reply"),
                button_node,
                BorderColor::all(style::BORDER),
                coms_button(font, "No", false),
            ))
            .observe(no_clicked);
        }
        ComsChannelType::Player => {
            p.spawn((
                Name::new("Coms message input"),
                UiComsMessage,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..Default::default()
                },
                BackgroundColor(style::SURFACE),
                BorderColor::all(style::BORDER),
                style::font(font, 15.0),
                TextColor(style::TEXT),
                PlaceholderText {
                    text: "Your message...".to_owned(),
                    color: style::MUTED,
                },
                TextInput {
                    input_type: InputType::Text { max_length: Some(100) },
                    text_node: Node::default(),
                },
            ));
            let mut button = coms_button(font, "Send message", true);
            button.submit_control = Some(CosmosInputs::SendComs);
            p.spawn((button_node, BorderColor::all(style::ACCENT), button)).observe(send_text);
        }
    }
}

fn create_messages_ui(font: &Handle<Font>, p: &mut ChildSpawnerCommands, messages: &[ComsMessage], your_ship: Entity) {
    if messages.is_empty() {
        style::label(p, font, "Waiting for a response...", 15.0, style::MUTED);
    }
    for message in messages {
        p.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            flex_shrink: 0.0,
            ..Default::default()
        })
        .with_children(|p| {
            style::label(
                p,
                font,
                if message.sender == your_ship { "YOU" } else { "TRANSMISSION" },
                11.0,
                style::MUTED,
            );
            style::label(p, font, &message.text, 16.0, style::TEXT);
        });
    }
}

#[derive(Component)]
struct MessageArea;

#[derive(Resource, Debug)]
pub struct ComsAssets {
    open: Handle<Image>,
    close: Handle<Image>,
}

#[derive(Component, Debug)]
struct ToggleButton;

fn toggle_coms_key(inputs: InputChecker, q_toggle: Query<Entity, With<ToggleButton>>, mut commands: Commands) {
    if inputs.check_just_pressed(CosmosInputs::ToggleComs)
        && let Ok(button) = q_toggle.single()
    {
        commands.trigger(ButtonEvent(button));
    }
}

fn on_not_pilot(
    mut commands: Commands,
    mut q_coms_ui: Query<(Entity, &mut Node, Has<ShowCursor>), With<ComsUi>>,
    coms_assets: Res<ComsAssets>,
    mut q_toggle_button: Query<&mut CosmosButton, With<ToggleButton>>,
    mut focused: ResMut<InputFocus>,
    q_coms: Query<(Entity, &ChildOf, &ComsChannel)>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_pilot: Query<&Pilot>,
    q_lp_not_pilot: Query<(), (With<LocalPlayer>, Without<Pilot>)>,
) {
    if q_lp_not_pilot.is_empty() {
        return;
    }
    let Ok((entity, mut node, has)) = q_coms_ui.single_mut() else {
        return;
    };
    if has {
        minimize_ui(
            &mut commands,
            &coms_assets,
            &mut q_toggle_button,
            entity,
            &mut node,
            &mut focused,
            &q_coms,
            &q_local_player,
            &q_pilot,
        );
    }
}

fn on_toggle(
    _trigger: On<ButtonEvent>,
    mut commands: Commands,
    mut q_selected_coms: Query<&mut SelectedComs>,
    mut q_coms_ui: Query<(Entity, &mut Node, Has<ShowCursor>), With<ComsUi>>,
    coms_assets: Res<ComsAssets>,
    mut q_toggle_button: Query<&mut CosmosButton, With<ToggleButton>>,
    mut focused: ResMut<InputFocus>,
    q_coms: Query<(Entity, &ChildOf, &ComsChannel)>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_pilot: Query<&Pilot>,
) {
    let Ok(lp) = q_local_player.single() else {
        return;
    };
    if !q_pilot.contains(lp) {
        return;
    }

    for (entity, mut node, has) in q_coms_ui.iter_mut() {
        if has {
            minimize_ui(
                &mut commands,
                &coms_assets,
                &mut q_toggle_button,
                entity,
                &mut node,
                &mut focused,
                &q_coms,
                &q_local_player,
                &q_pilot,
            );
        } else {
            if let Ok(mut selected) = q_selected_coms.single_mut()
                && !q_coms.contains(selected.0)
            {
                let all_coms = get_all_coms(&q_coms, &q_pilot, &q_local_player);
                let Some(first) = all_coms.first() else {
                    continue;
                };
                selected.0 = first.0;
            }
            node.right = Val::Px(12.0);
            commands
                .entity(entity)
                .insert((ShowCursor, OpenMenu::with_close_method(0, CloseMethod::Custom)));

            if let Ok(mut tb) = q_toggle_button.single_mut() {
                tb.image = Some(ImageNode::new(coms_assets.close.clone()));
            }
        }
    }
}

fn minimize_ui(
    commands: &mut Commands,
    coms_assets: &ComsAssets,
    q_toggle_button: &mut Query<&mut CosmosButton, With<ToggleButton>>,
    entity: Entity,
    node: &mut Node,
    focused: &mut InputFocus,
    q_coms: &Query<(Entity, &ChildOf, &ComsChannel)>,
    q_local_player: &Query<Entity, With<LocalPlayer>>,
    q_pilot: &Query<&Pilot>,
) {
    let Val::Px(w) = node.width else {
        return;
    };

    let all_coms = get_all_coms(q_coms, q_pilot, q_local_player);

    let on_screen_amt = if all_coms.is_empty() { -COMS_TAB_WIDTH } else { COMS_TAB_WIDTH };

    node.right = Val::Px(-w + on_screen_amt);
    commands.entity(entity).remove::<ShowCursor>().remove::<OpenMenu>();
    focused.0 = None;
    if let Ok(mut tb) = q_toggle_button.single_mut() {
        tb.image = Some(ImageNode::new(coms_assets.open.clone()));
    }
}

// Use the laid-out width so the reopen tab remains reachable in narrow windows.
fn position_minimized_ui(mut q_ui: Query<(&ComputedNode, &mut Node), (With<ComsUi>, Without<ShowCursor>)>) {
    for (computed, mut node) in &mut q_ui {
        let width = computed.size().x * computed.inverse_scale_factor();
        if width > 0.0 {
            node.right = Val::Px(-width + COMS_TAB_WIDTH);
        }
    }
}

// Wait for the new text to be measured before asking ScrollBox to show its end.
fn scroll_to_latest(mut commands: Commands, mut q_scroll: Query<(Entity, &mut ScrollBox), With<ScrollToLatest>>) {
    for (entity, mut scroll) in &mut q_scroll {
        scroll.scroll_amount = Val::Percent(100.0);
        commands.entity(entity).remove::<ScrollToLatest>();
    }
}

fn on_close_menu(
    coms_assets: Res<ComsAssets>,
    mut evr: MessageReader<CloseMenuMessage>,
    mut q_coms_ui: Query<&mut Node, With<ComsUi>>,
    mut commands: Commands,
    mut q_toggle_button: Query<&mut CosmosButton, With<ToggleButton>>,
    mut focused: ResMut<InputFocus>,
    q_coms: Query<(Entity, &ChildOf, &ComsChannel)>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_pilot: Query<&Pilot>,
) {
    for ev in evr.read() {
        if let Ok(mut node) = q_coms_ui.get_mut(ev.0) {
            minimize_ui(
                &mut commands,
                &coms_assets,
                &mut q_toggle_button,
                ev.0,
                &mut node,
                &mut focused,
                &q_coms,
                &q_local_player,
                &q_pilot,
            );
        }
    }
}

fn send_text(
    _trigger: On<ButtonEvent>,
    mut nevw_send_coms_message: NettyMessageWriter<SendComsMessage>,
    q_selected_coms: Query<&SelectedComs, With<ShowCursor>>,
    mut q_text_value: Query<&mut InputValue, With<UiComsMessage>>,
    q_coms_channel: Query<&ComsChannel>,
) {
    let Ok(mut text) = q_text_value.single_mut() else {
        return;
    };

    let Ok(selected) = q_selected_coms.single() else {
        return;
    };

    let val = text.value();

    if val.is_empty() {
        return;
    }

    let Ok(coms_channel) = q_coms_channel.get(selected.0) else {
        return;
    };

    nevw_send_coms_message.write(SendComsMessage {
        message: SendComsMessageType::Message(val.to_owned()),
        to: coms_channel.with,
    });

    *text = Default::default();
}

fn yes_clicked(
    _trigger: On<ButtonEvent>,
    mut nevw_send_coms_message: NettyMessageWriter<SendComsMessage>,
    q_selected_coms: Query<&SelectedComs, With<ShowCursor>>,
    q_coms_channel: Query<&ComsChannel>,
) {
    let Ok(selected) = q_selected_coms.single() else {
        return;
    };

    let Ok(coms_channel) = q_coms_channel.get(selected.0) else {
        return;
    };

    nevw_send_coms_message.write(SendComsMessage {
        message: SendComsMessageType::Yes,
        to: coms_channel.with,
    });
}

fn no_clicked(
    _trigger: On<ButtonEvent>,
    mut nevw_send_coms_message: NettyMessageWriter<SendComsMessage>,
    q_selected_coms: Query<&SelectedComs, With<ShowCursor>>,
    q_coms_channel: Query<&ComsChannel>,
) {
    let Ok(selected) = q_selected_coms.single() else {
        return;
    };

    let Ok(coms_channel) = q_coms_channel.get(selected.0) else {
        return;
    };

    nevw_send_coms_message.write(SendComsMessage {
        message: SendComsMessageType::No,
        to: coms_channel.with,
    });
}

fn end_selected_coms(
    _trigger: On<ButtonEvent>,
    mut evw_close_coms: NettyMessageWriter<RequestCloseComsMessage>,
    mut q_selected_coms: Query<&mut SelectedComs>,
) {
    let Ok(selected) = q_selected_coms.single_mut() else {
        return;
    };

    evw_close_coms.write(RequestCloseComsMessage(selected.0));
}

fn on_left_clicked(
    _trigger: On<ButtonEvent>,
    mut q_selected_coms: Query<&mut SelectedComs>,
    q_coms: Query<(Entity, &ChildOf, &ComsChannel)>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_pilot: Query<&Pilot>,
) {
    let Ok(mut selected) = q_selected_coms.single_mut() else {
        return;
    };

    let all_coms = get_all_coms(&q_coms, &q_pilot, &q_local_player);

    if let Some([prev, _]) = all_coms.array_windows::<2>().find(|[_, b]| b.0 == selected.0) {
        selected.0 = prev.0;
    } else if let Some(last) = all_coms.last()
        && selected.0 != last.0
    {
        selected.0 = last.0;
    }
}

fn on_right_clicked(
    _trigger: On<ButtonEvent>,
    mut q_selected_coms: Query<&mut SelectedComs>,
    q_coms: Query<(Entity, &ChildOf, &ComsChannel)>,
    q_pilot: Query<&Pilot>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
) {
    let Ok(mut selected) = q_selected_coms.single_mut() else {
        return;
    };

    let all_coms = get_all_coms(&q_coms, &q_pilot, &q_local_player);

    if let Some([_, next]) = all_coms.array_windows::<2>().find(|[a, _]| a.0 == selected.0) {
        selected.0 = next.0;
    } else if let Some(first) = all_coms.first()
        && selected.0 != first.0
    {
        selected.0 = first.0;
    }
}

fn get_all_coms<'a>(
    q_coms: &'a Query<(Entity, &ChildOf, &ComsChannel)>,
    q_pilot: &'a Query<&Pilot>,
    q_local_player: &'a Query<Entity, With<LocalPlayer>>,
) -> Vec<(Entity, &'a ChildOf, &'a ComsChannel)> {
    let Ok(lp) = q_local_player.single() else {
        return vec![];
    };
    let Ok(pilot) = q_pilot.get(lp) else {
        return vec![];
    };
    let lp_piloting_ship = pilot.entity;

    let mut channels = q_coms
        .iter()
        .filter(|(_, parent, _)| parent.parent() == lp_piloting_ship)
        .collect::<Vec<_>>();
    channels.sort_unstable_by_key(|(entity, _, _)| entity.to_bits());
    channels
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (sync_coms_ui, update_contact_details, on_not_pilot, on_close_menu, toggle_coms_key)
            .chain()
            .in_set(UiSystemSet::PreDoUi)
            .run_if(in_state(GameState::Playing)),
    );
    app.add_systems(
        PostUpdate,
        (position_minimized_ui, scroll_to_latest).after(bevy::ui::UiSystems::Layout),
    );

    load_assets::<Image, ComsAssets, 2>(
        app,
        GameState::Loading,
        ["cosmos/images/ui/open-coms.png", "cosmos/images/ui/close-coms.png"],
        |mut cmds, [(open, _), (close, _)]| cmds.insert_resource(ComsAssets { open, close }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::coms::AiComsType;

    fn setup() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.insert_resource(DefaultFont(Handle::default()))
            .insert_resource(ComsAssets {
                open: Handle::default(),
                close: Handle::default(),
            })
            .init_resource::<InputFocus>()
            .add_systems(Update, sync_coms_ui);
        let ship = app.world_mut().spawn_empty().id();
        app.world_mut().spawn((LocalPlayer, Pilot { entity: ship }));
        let contact = app.world_mut().spawn_empty().id();
        let channel = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ComsChannel {
                    with: contact,
                    channel_type: ComsChannelType::Player,
                    messages: vec![],
                },
            ))
            .id();
        app.update();
        app.update();
        (app, ship, channel)
    }

    #[test]
    fn incoming_messages_preserve_the_player_draft() {
        let (mut app, ship, channel) = setup();
        let input = app
            .world_mut()
            .query_filtered::<Entity, With<UiComsMessage>>()
            .single(app.world())
            .unwrap();
        app.world_mut().get_mut::<InputValue>(input).unwrap().set_value("Unsent draft");
        app.world_mut().get_mut::<ComsChannel>(channel).unwrap().messages.push(ComsMessage {
            text: "New transmission".into(),
            sender: ship,
        });
        app.update();
        assert_eq!(app.world().get::<InputValue>(input).unwrap().value(), "Unsent draft");
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == "New transmission")
        );
        assert!(app.world_mut().query::<&Text>().iter(app.world()).any(|text| text.0 == "YOU"));
    }

    #[test]
    fn switching_channel_types_and_closing_channels_updates_the_controls() {
        let (mut app, ship, player_channel) = setup();
        let ai_channel = app
            .world_mut()
            .spawn((
                ChildOf(ship),
                ComsChannel {
                    with: ship,
                    channel_type: ComsChannelType::Ai(AiComsType::YesNo),
                    messages: vec![],
                },
            ))
            .id();
        let root = app
            .world_mut()
            .query_filtered::<Entity, With<ComsUi>>()
            .single(app.world())
            .unwrap();
        app.world_mut().get_mut::<SelectedComs>(root).unwrap().0 = ai_channel;
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<UiComsMessage>>()
                .iter(app.world())
                .count(),
            0
        );
        let buttons = app
            .world_mut()
            .query::<&CosmosButton>()
            .iter(app.world())
            .filter_map(|button| button.text.as_ref().map(|t| t.0.clone()))
            .collect::<Vec<_>>();
        assert!(buttons.iter().any(|text| text == "Yes"));
        assert!(buttons.iter().any(|text| text == "No"));
        app.world_mut().despawn(ai_channel);
        app.update();
        assert_eq!(app.world().get::<SelectedComs>(root).unwrap().0, player_channel);
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<UiComsMessage>>()
                .iter(app.world())
                .count(),
            1
        );
        app.world_mut().despawn(player_channel);
        app.update();
        assert!(app.world().get_entity(root).is_err());
        assert!(app.world().resource::<InputFocus>().0.is_none());
    }

    #[test]
    fn keyboard_toggle_reopens_a_minimized_panel() {
        use crate::input::inputs::CosmosInputHandler;

        let (mut app, _, _) = setup();
        let mut input = CosmosInputHandler::default();
        input.set_keycode(CosmosInputs::ToggleComs, KeyCode::Backquote);
        app.insert_resource(input)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, toggle_coms_key);
        let root = app
            .world_mut()
            .query_filtered::<Entity, With<ComsUi>>()
            .single(app.world())
            .unwrap();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Backquote);
        app.update();
        assert!(app.world().get::<ShowCursor>(root).is_none());
        assert!(app.world().get::<OpenMenu>(root).is_none());
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().reset_all();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Backquote);
        app.update();
        assert!(app.world().get::<ShowCursor>(root).is_some());
        assert!(app.world().get::<OpenMenu>(root).is_some());
    }
}
