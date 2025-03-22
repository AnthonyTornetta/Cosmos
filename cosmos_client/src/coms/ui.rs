use crate::{
    asset::asset_loader::load_assets,
    ui::{
        components::{
            button::{register_button, ButtonEvent, CosmosButton},
            text_input::{InputType, TextInput},
        },
        font::DefaultFont,
    },
};
use bevy::{color::palettes::css, prelude::*};
use cosmos_core::coms::ComsChannel;
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::state::GameState;
use cosmos_core::structure::ship::pilot::Pilot;

#[derive(Component)]
struct ComsUi;

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

    let Ok(pilot) = q_pilot.get(parent.get()) else {
        return;
    };

    let Ok(local_player) = q_local_player.get(pilot.entity) else {
        return;
    };

    if pilot.entity != local_player {
        return;
    }

    let messages = coms.messages.iter().map(|x| x.text.as_str()).collect::<Vec<_>>();
    if !q_coms_ui.is_empty() {
        commands
            .entity(q_body.get_single().expect("Body missing"))
            .despawn_descendants()
            .with_children(|p| create_messages_ui(&font, p, &messages));

        text.0 = "[Ship Name]".into();
    } else {
        create_coms_ui(&mut commands, &coms_assets, &font, coms_ent, &messages);
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

        if !q_local_player.contains(pilot.entity) {
            continue;
        }

        // Write code to assert that 1=1 using assert_eq

        let messages = coms.messages.iter().map(|x| x.text.as_str()).collect::<Vec<_>>();
        if !q_coms_ui.is_empty() {
            let (mut coms, mut text) = q_header.get_single_mut().expect("Header missing");
            if coms.0 != coms_ent {
                continue;
            }

            commands
                .entity(q_body.get_single().expect("Body missing"))
                .despawn_descendants()
                .with_children(|p| create_messages_ui(&font, p, &messages));

            coms.0 = coms_ent;
            text.0 = "[Ship Name]".into();
        } else {
            create_coms_ui(&mut commands, &coms_assets, &font, coms_ent, &messages);
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
        } else {
            if let Ok(coms_ui_ent) = q_coms_ui.get_single() {
                commands.entity(coms_ui_ent).despawn_recursive();
            }
        }
    }
}

fn create_coms_ui(commands: &mut Commands, coms_assets: &ComsAssets, font: &DefaultFont, current_coms_ent: Entity, messages: &[&str]) {
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
                    ImageNode::new(coms_assets.close.clone_weak()),
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
                        Name::new("Messages"),
                        MessageArea,
                        Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| {
                        create_messages_ui(&message_font.font, p, messages);
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
                    .with_children(|p| {
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
                    });
                });
            });
        });
}

fn create_messages_ui(font: &Handle<Font>, messages_container: &mut ChildBuilder, messages: &[&str]) {
    let message_font = TextFont {
        font: font.clone_weak(),
        font_size: 20.0,
        ..Default::default()
    };

    let accent = css::AQUA.into();

    for msg in messages {
        messages_container.spawn((
            Name::new("Message"),
            Text::new(*msg),
            Node {
                padding: UiRect::all(Val::Px(10.0)),
                margin: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            message_font.clone(),
            BorderColor(accent),
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

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_remove_coms, on_change_coms, on_change_selected_coms)
            .chain()
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between),
    );

    register_button::<LeftClicked>(app);
    register_button::<RightClicked>(app);
    register_button::<SendClicked>(app);

    load_assets::<Image, ComsAssets, 2>(
        app,
        GameState::Loading,
        ["cosmos/images/ui/open-coms.png", "cosmos/images/ui/close-coms.png"],
        |mut cmds, [(open, _), (close, _)]| cmds.insert_resource(ComsAssets { open, close }),
    );
}
