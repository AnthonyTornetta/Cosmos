use std::{
    env,
    io::{BufRead, BufReader},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{color::palettes::css, ecs::relationship::RelatedSpawnerCommands, prelude::*};
use cosmos_core::state::GameState;
use derive_more::{Display, Error};
use walkdir::WalkDir;

use crate::{
    netty::{connect::ConnectToConfig, steam::User},
    ui::{
        components::{
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            modal::confirm_modal::{ConfirmModal, ConfirmModalComplete, TextModalButtons},
            scollable_container::ScrollBox,
            text_input::{InputValue, TextInput},
            window::WindowAssets,
        },
        font::DefaultFont,
        main_menu::{MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet, in_main_menu_state},
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
    },
};

#[derive(Component)]
struct CreateWorldUi;

#[derive(Debug, Clone, Component, PartialEq, Eq, Default)]
struct WorldNameText(String);
impl ReactableValue for WorldNameText {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.to_owned();
    }
}

#[derive(Debug, Clone, Component, PartialEq, Eq, Default)]
struct SeedText(String);
impl ReactableValue for SeedText {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.to_owned();
    }
}

#[derive(Debug, Clone, Component, PartialEq, Eq, Default)]
struct WorldNameErrorMessage(String);

impl ReactableValue for WorldNameErrorMessage {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.to_owned();
    }
}

fn create_singleplayer_screen(
    mut commands: Commands,
    q_ui_root: Query<Entity, With<MainMenuRootUiNode>>,
    font: Res<DefaultFont>,
    client: Res<User>,
) {
    info!("creating singeplayer screen!");
    let Ok(main_menu_root) = q_ui_root.single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        create_menu(p, &font, &client);
    });
}

fn create_menu(p: &mut RelatedSpawnerCommands<ChildOf>, default_font: &DefaultFont, _client: &User) {
    let _text_style_small = TextFont {
        font_size: 24.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap();

    let _text_style = TextFont {
        font_size: 32.0,
        font: default_font.get(),
        ..Default::default()
    };

    let bg = BackgroundColor(Srgba::hex("#111111").unwrap().into());

    p.spawn((
        Name::new("Header"),
        Node {
            height: Val::Px(100.0),
            width: Val::Percent(100.0),
            ..Default::default()
        },
        bg,
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("Select World"),
            Node {
                margin: UiRect::all(Val::Auto),
                ..Default::default()
            },
            TextFont {
                font: default_font.get(),
                font_size: 52.0,
                ..Default::default()
            },
        ));
    });

    p.spawn((
        ScrollBox { ..Default::default() },
        Node {
            flex_grow: 1.0,
            margin: UiRect::horizontal(Val::Auto),
            width: Val::Percent(100.0),
            max_width: Val::Px(800.0),
            ..Default::default()
        },
    ))
    .with_children(|p| {
        p.spawn(Node {
            flex_grow: 1.0,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|p| {
            let existing_worlds = WalkDir::new("./worlds/")
                .max_depth(1)
                .into_iter()
                .flatten()
                .skip(1) // The first is always the root "worlds" folder
                .filter(|x| x.file_type().is_dir())
                .map(|x| x.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>();

            if existing_worlds.is_empty() {
                p.spawn((
                    Text::new("No Worlds :("),
                    TextFont {
                        font: default_font.get(),
                        font_size: 40.0,
                        ..Default::default()
                    },
                    Node {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..Default::default()
                    },
                ));

                p.spawn((
                    Text::new("Create One Below"),
                    TextFont {
                        font: default_font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                    Node {
                        margin: UiRect::all(Val::Px(10.0)),
                        ..Default::default()
                    },
                ));
            } else {
                for world in existing_worlds {
                    let mut ecmds = p.spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(100.0),
                        ..Default::default()
                    });

                    let world_entry_entity = ecmds.id();

                    ecmds.with_children(|p| {
                        let world_moved = world.clone();
                        p.spawn((
                            CosmosButton {
                                text: Some((
                                    "PLAY".into(),
                                    TextFont {
                                        font: default_font.get(),
                                        font_size: 24.0,
                                        ..Default::default()
                                    },
                                    Default::default(),
                                )),
                                ..Default::default()
                            },
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                margin: UiRect {
                                    left: Val::ZERO,
                                    right: Val::Px(50.0),
                                    top: Val::Auto,
                                    bottom: Val::Auto,
                                },
                                ..Default::default()
                            },
                        ))
                        .observe(
                            move |_: On<ButtonEvent>, commands: Commands, state: ResMut<NextState<GameState>>| {
                                let port = start_server_for_world(&world_moved, None).expect("Couldn't start existing world?");
                                trigger_connection(port, state, commands);
                            },
                        );

                        p.spawn((
                            Node {
                                margin: UiRect::vertical(Val::Auto),
                                ..Default::default()
                            },
                            Text::new(world.replace("_", " ")),
                            TextFont {
                                font: default_font.get(),
                                font_size: 32.0,
                                ..Default::default()
                            },
                        ));

                        let world_moved = world.clone();
                        p.spawn((
                            CosmosButton {
                                text: Some((
                                    "DELETE".into(),
                                    TextFont {
                                        font: default_font.get(),
                                        font_size: 24.0,
                                        ..Default::default()
                                    },
                                    Default::default(),
                                )),
                                ..Default::default()
                            },
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                margin: UiRect {
                                    left: Val::Auto,
                                    right: Val::Px(50.0),
                                    top: Val::Auto,
                                    bottom: Val::Auto,
                                },
                                ..Default::default()
                            },
                        ))
                        .observe(move |_: On<ButtonEvent>, mut commands: Commands| {
                            let prompt = format!("Are you sure you want to delete {}?", world_moved.replace("_", " "));
                            let world = world_moved.clone();
                            commands
                                .spawn(ConfirmModal {
                                    prompt,
                                    buttons: TextModalButtons::YesNo,
                                })
                                .observe(move |_: On<ConfirmModalComplete>, mut commands: Commands| {
                                    if let Err(e) = trash::delete(format!("worlds/{world}")) {
                                        error!("Error deleting world {world} - {e:?}");
                                    }
                                    commands.entity(world_entry_entity).despawn();
                                });
                        });
                    });
                }
            }
        });
    });

    p.spawn((
        bg,
        Name::new("Bottom Bar"),
        Node {
            padding: UiRect::all(Val::Px(20.0)),
            width: Val::Percent(100.0),
            align_content: AlignContent::Center,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
    ))
    .with_children(|p| {
        p.spawn((
            Name::new("Cancel"),
            BorderColor::all(cool_blue),
            Node {
                margin: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                ..Default::default()
            },
            CosmosButton {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some((
                    "Cancel".into(),
                    TextFont {
                        font: default_font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                    Default::default(),
                )),
                ..Default::default()
            },
        ))
        .observe(|_: On<ButtonEvent>, mut mms: ResMut<MainMenuSubState>| *mms = MainMenuSubState::TitleScreen);

        p.spawn((
            Name::new("Create World"),
            BorderColor::all(cool_blue),
            Node {
                margin: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                ..Default::default()
            },
            CosmosButton {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some((
                    "Create World".into(),
                    TextFont {
                        font: default_font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                    Default::default(),
                )),
                ..Default::default()
            },
        ))
        .observe(
            |_: On<ButtonEvent>,
             q_already_exists: Query<(), With<CreateWorldUi>>,
             mut commands: Commands,
             default_font: Res<DefaultFont>,
             window_assets: Res<WindowAssets>| {
                if !q_already_exists.is_empty() {
                    return;
                }

                let mut ecmds = commands.spawn((
                    CreateWorldUi,
                    BackgroundColor(Srgba::hex("#333333").unwrap().into()),
                    BorderColor::all(Srgba::hex("#111111").unwrap()),
                    Node {
                        position_type: PositionType::Absolute,
                        margin: UiRect::all(Val::Auto),
                        width: Val::Percent(100.0),
                        max_width: Val::Px(800.0),
                        max_height: Val::Px(800.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(2.0)),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    Name::new("Create World Modal"),
                    WorldNameErrorMessage::default(),
                    WorldNameText("New World".into()),
                    SeedText("".into()),
                ));

                let window_ent = ecmds.id();

                ecmds.with_children(|p| {
                    let mut title_bar = p.spawn((
                        Name::new("Title Bar"),
                        Interaction::None,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            width: Val::Percent(100.0),
                            height: Val::Px(60.0),
                            padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(0.0), Val::Px(0.0)),

                            ..default()
                        },
                        BackgroundColor(css::WHITE.into()),
                        ImageNode::new(window_assets.title_bar_image.clone()),
                    ));

                    title_bar.with_children(|parent| {
                        parent.spawn((
                            Name::new("Title Text"),
                            Text::new("Create World"),
                            TextFont {
                                font_size: 24.0,
                                font: default_font.clone(),
                                ..Default::default()
                            },
                            TextLayout {
                                justify: Justify::Center,
                                ..Default::default()
                            },
                        ));
                    });

                    p.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        margin: UiRect::all(Val::Px(10.0)),
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn((
                            Text::new("World Name"),
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                            Node {
                                margin: UiRect::vertical(Val::Px(10.0)),
                                ..Default::default()
                            },
                        ));
                        p.spawn((
                            BindValues::<WorldNameText>::new(vec![BindValue::new(window_ent, ReactableFields::Value)]),
                            TextInput { ..Default::default() },
                            InputValue::new("New World"),
                            BackgroundColor(Srgba::hex("#222222").unwrap().into()),
                            BorderColor::all(css::GREY),
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                width: Val::Percent(100.0),
                                margin: UiRect::vertical(Val::Px(10.0)),
                                ..Default::default()
                            },
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            BindValues::<WorldNameErrorMessage>::new(vec![
                                BindValue::new(window_ent, ReactableFields::Text { section: 0 }),
                                BindValue::new(
                                    window_ent,
                                    ReactableFields::Visibility {
                                        hidden_value: "".into(),
                                        visibile_value: Display::Flex,
                                    },
                                ),
                            ]),
                            Text::new(""),
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                            TextColor(css::RED.into()),
                            Node {
                                margin: UiRect::vertical(Val::Px(10.0)),
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            Text::new("Seed"),
                            Node {
                                margin: UiRect::vertical(Val::Px(10.0)),
                                ..Default::default()
                            },
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                        ));
                        p.spawn((
                            BindValues::<SeedText>::new(vec![BindValue::new(window_ent, ReactableFields::Value)]),
                            TextInput { ..Default::default() },
                            InputValue::new(""),
                            BackgroundColor(Srgba::hex("#222222").unwrap().into()),
                            BorderColor::all(css::GREY),
                            Node {
                                padding: UiRect::all(Val::Px(8.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                width: Val::Percent(100.0),
                                margin: UiRect::vertical(Val::Px(10.0)),
                                ..Default::default()
                            },
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                        ));

                        p.spawn(Node {
                            width: Val::Percent(100.0),
                            margin: UiRect::top(Val::Auto),
                            ..Default::default()
                        })
                        .with_children(|p| {
                            p.spawn((
                                Node {
                                    margin: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    flex_grow: 1.0,
                                    height: Val::Px(70.0),
                                    ..Default::default()
                                },
                                BorderColor::all(css::GREY),
                                CosmosButton {
                                    button_styles: Some(ButtonStyles {
                                        background_color: Srgba::hex("232323").unwrap().into(),
                                        hover_background_color: Srgba::hex("222222").unwrap().into(),
                                        press_background_color: Srgba::hex("111111").unwrap().into(),
                                        ..Default::default()
                                    }),
                                    text: Some((
                                        "Cancel".into(),
                                        TextFont {
                                            font: default_font.get(),
                                            font_size: 24.0,
                                            ..Default::default()
                                        },
                                        Default::default(),
                                    )),
                                    ..Default::default()
                                },
                            ))
                            .observe(move |_: On<ButtonEvent>, mut commands: Commands| {
                                commands.entity(window_ent).despawn();
                            });

                            p.spawn((
                                Node {
                                    margin: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    flex_grow: 1.0,
                                    height: Val::Px(70.0),
                                    ..Default::default()
                                },
                                BorderColor::all(css::GREY),
                                CosmosButton {
                                    button_styles: Some(ButtonStyles {
                                        background_color: Srgba::hex("232323").unwrap().into(),
                                        hover_background_color: Srgba::hex("222222").unwrap().into(),
                                        press_background_color: Srgba::hex("111111").unwrap().into(),
                                        ..Default::default()
                                    }),
                                    text: Some((
                                        "Create".into(),
                                        TextFont {
                                            font: default_font.get(),
                                            font_size: 24.0,
                                            ..Default::default()
                                        },
                                        Default::default(),
                                    )),
                                    ..Default::default()
                                },
                            ))
                            .observe(
                                move |_: On<ButtonEvent>,
                                      state: ResMut<NextState<GameState>>,
                                      mut commands: Commands,
                                      mut q_error_message: Query<&mut WorldNameErrorMessage>,
                                      q_seed_text: Query<&SeedText>,
                                      q_world_name_text: Query<&WorldNameText>| {
                                    let seed = q_seed_text.single().cloned().unwrap_or_default();
                                    let world_name = q_world_name_text.single().cloned().unwrap_or_default();

                                    match start_server_for_world(
                                        &world_name.0,
                                        if seed.0.is_empty() { None } else { Some(seed.0.as_str()) },
                                    ) {
                                        Err(e) => {
                                            for mut msg in q_error_message.iter_mut() {
                                                match e {
                                                    WorldStartError::EmptyWorldName => {
                                                        msg.0 = "World name cannot be empty".to_string();
                                                    }
                                                    WorldStartError::WorldNameTooLong => {
                                                        msg.0 = "World name cannot exceed 40 characters".to_string();
                                                    }
                                                    WorldStartError::InvalidName(c) => {
                                                        msg.0 = format!("World name cannot contain '{c}'");
                                                    }
                                                    WorldStartError::CouldNotFindPort => {
                                                        msg.0 = "Error starting server (port error). Please try again".into();
                                                    }
                                                    WorldStartError::MissingServerExecutable => {
                                                        msg.0 =
                                                            "Could not find server executable file. Please verify your installation".into()
                                                    }
                                                }
                                            }
                                            error!("{e:?}");
                                        }
                                        Ok(port) => {
                                            commands.entity(window_ent).despawn();
                                            trigger_connection(port, state, commands);
                                        }
                                    }
                                },
                            );
                        });
                    });
                });
            },
        );
    });
}

#[derive(Debug, Error, Display)]
enum WorldStartError {
    EmptyWorldName,
    WorldNameTooLong,
    InvalidName(#[error(not(source))] char),
    MissingServerExecutable,
    CouldNotFindPort,
}

fn find_invalid_char(s: &str) -> Option<char> {
    s.chars().find(|&c| !c.is_alphanumeric() && c != '-' && c != '_')
}

fn start_server_for_world(world_name: &str, seed: Option<&str>) -> Result<u16, WorldStartError> {
    let world_name = world_name.replace(" ", "_");

    if world_name.is_empty() {
        return Err(WorldStartError::EmptyWorldName);
    }

    if world_name.len() > 40 {
        return Err(WorldStartError::WorldNameTooLong);
    }

    if let Some(c) = find_invalid_char(&world_name) {
        return Err(WorldStartError::InvalidName(c));
    }

    // find server bin
    use std::process::{Command, Stdio};

    // Determine the correct executable name.
    let server_executable = if cfg!(target_os = "windows") {
        "cosmos_server.exe"
    } else {
        "cosmos_server"
    };

    let working_dir = env::current_dir().unwrap();

    let mut server_path = working_dir.clone();
    server_path.pop();
    server_path.push("cosmos_server/");
    server_path.push(server_executable);

    let mut dev_mode = false;

    if !server_path.exists() {
        // Also check for build files
        #[cfg(debug_assertions)]
        {
            server_path = working_dir.clone();
            server_path.pop();
            server_path.push("target/debug");
            dev_mode = true;
        }
        #[cfg(not(debug_assertions))]
        {
            server_path = working_dir.clone();
            server_path.pop();
            server_path.push("target/release");
            info!("{server_path:?}");
            dev_mode = true;
        }
        info!("bad? {server_path:?}");
        server_path.push(server_executable);
        info!("{server_path:?}");
    }

    if !server_path.exists() {
        error!("{server_path:?}");
        return Err(WorldStartError::MissingServerExecutable);
    }

    let mut cmd = Command::new(server_path);
    if dev_mode {
        let mut dir = working_dir.clone();
        dir.pop();
        dir.push("cosmos_server/");
        cmd.current_dir(dir);
    } else {
        let mut dir = working_dir.clone();
        dir.push("server/");
        cmd.current_dir(dir);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::inherit());

    let mut world_path = env::current_dir().unwrap();
    world_path.push("worlds/");
    world_path.push(world_name);

    cmd.arg("--world").arg(world_path.to_str().unwrap()).arg("--local");

    if let Some(seed) = seed {
        cmd.arg("--seed").arg(seed);
    }

    let mut child = match cmd.spawn() {
        Err(e) => {
            error!("{e:?}");
            return Err(WorldStartError::MissingServerExecutable);
        }
        Ok(c) => {
            info!("Starting server - pid = {}", c.id());
            c
        }
    };

    let child_stdout = child.stdout.take().expect("Failed to open stdin");
    let child_stderr = child.stderr.take().expect("Failed to open stdin");

    let port_mutex: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));

    std::thread::spawn(move || {
        for line in BufReader::new(child_stderr).lines() {
            let Ok(line) = line else {
                break;
            };

            println!("<server> {line}");
        }
    });

    let moved_mutex = port_mutex.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(child_stdout).lines() {
            let Ok(line) = line else {
                break;
            };

            println!("<server> {line}");

            if line.contains("Port: ") {
                let mut splt = line.split("Port: ");
                splt.next();
                let port = splt.next().expect("Checked about").trim();
                match port.parse::<u16>() {
                    Err(e) => {
                        error!("Bad port message from server! {e:?}");
                    }
                    Ok(p) => {
                        *moved_mutex.lock().unwrap() = Some(p);
                    }
                }
            }
        }
    });

    // ~2sec of waiting
    const MAX_TRIED: u32 = 200;
    let mut count = 0;
    info!("Waiting on port from server...");
    while port_mutex.lock().unwrap().is_none() {
        count += 1;
        std::thread::sleep(Duration::from_millis(10));

        if count >= MAX_TRIED {
            break;
        }
    }

    if let Some(port) = *port_mutex.lock().unwrap() {
        info!("Got port {port}");
        Ok(port)
    } else {
        info!("Timed out waiting for port -- killing server.");
        let _ = child.kill();
        Err(WorldStartError::CouldNotFindPort)
    }
}

fn trigger_connection(port: u16, mut state: ResMut<NextState<GameState>>, mut commands: Commands) {
    let con_str = format!("127.0.0.1:{port}");

    let host_cfg = con_str.parse::<SocketAddr>().map(ConnectToConfig::Ip).unwrap();

    info!("Successful parsing of host! Starting connection process for {host_cfg:?}");

    commands.insert_resource(host_cfg);
    state.set(GameState::Connecting);
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<WorldNameErrorMessage>(app);
    add_reactable_type::<WorldNameText>(app);
    add_reactable_type::<SeedText>(app);

    app.add_systems(
        Update,
        create_singleplayer_screen
            .run_if(in_main_menu_state(MainMenuSubState::Singleplayer))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    );
}
