use std::{
    env, fs,
    io::{BufRead, BufReader, Read},
    net::SocketAddr,
    path::PathBuf,
    process,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{
    color::palettes::css,
    ecs::relationship::{RelatedSpawner, RelatedSpawnerCommands},
    prelude::*,
};
use bevy_renet::steam::steamworks::{ServerListCallbacks, ServerListRequest, ServerResponse, SteamId};
use cosmos_core::state::GameState;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::{
    netty::{connect::ConnectToConfig, steam::User},
    ui::{
        components::{
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            modal::{
                Modal,
                text_modal::{TextModal, TextModalButtons, TextModalComplete},
            },
            scollable_container::ScrollBox,
            text_input::{InputType, InputValue, TextInput},
        },
        font::DefaultFont,
        main_menu::{MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet, in_main_menu_state},
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
    },
};

#[derive(Debug, Clone, Component, PartialEq, Eq)]
struct ConnectionString(String);

impl ReactableValue for ConnectionString {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.to_owned();
    }
}

#[derive(Debug, Clone, Component, PartialEq, Eq, Default)]
struct ErrorMessage(String);

impl ReactableValue for ErrorMessage {
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

fn create_menu(p: &mut RelatedSpawnerCommands<ChildOf>, default_font: &DefaultFont, client: &User) {
    let text_style_small = TextFont {
        font_size: 24.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap();

    let text_style = TextFont {
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
        bg.clone(),
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
            width: Val::Percent(80.0),
            ..Default::default()
        },
    ));

    p.spawn((
        bg.clone(),
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
        ));

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
                    "Direct Connect".into(),
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
        .observe(|_: On<ButtonEvent>, mut commands: Commands, default_font: Res<DefaultFont>| {
            commands
                .spawn((
                    Modal {
                        title: "Create World".into(),
                    },
                    Name::new("Direct Connect Modal"),
                ))
                .with_children(|p| {
                    p.spawn(
                        (Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            ..Default::default()
                        }),
                    )
                    .with_children(|p| {
                        p.spawn((
                            Text::new("World Name"),
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                        ));
                        p.spawn((
                            Node {
                                width: Val::Percent(80.0),
                                ..Default::default()
                            },
                            TextInput {
                                input_type: InputType::Text { max_length: Some(30) },
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            Text::new("Seed"),
                            TextFont {
                                font: default_font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                        ));
                        p.spawn((
                            BorderColor::all(css::GREY),
                            Node {
                                display: Display::None,
                                flex_grow: 1.0,
                                padding: UiRect::all(Val::Px(5.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                margin: UiRect::horizontal(Val::Px(3.0)),
                                ..Default::default()
                            },
                            TextInput { ..Default::default() },
                        ));

                        p.spawn(
                            (Node {
                                width: Val::Percent(100.0),
                                ..Default::default()
                            }),
                        )
                        .with_children(|p| {
                            p.spawn((
                                Node {
                                    margin: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    flex_grow: 1.0,
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
                            ));

                            p.spawn((
                                Node {
                                    margin: UiRect::all(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    flex_grow: 1.0,
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
                                |_: On<ButtonEvent>, state: ResMut<NextState<GameState>>, commands: Commands| match start_server_for_world(
                                    "test", None,
                                ) {
                                    Err(e) => {
                                        error!("{e:?}");
                                    }
                                    Ok(port) => {
                                        trigger_connection(port, state, commands);
                                    }
                                },
                            );
                        });
                    });
                });
        });
    });
}

#[derive(Debug, Error, Display)]
enum WorldStartError {
    InvalidName(#[error(not(source))] char),
    MissingServerExecutable,
    CouldNotFindPort,
}

fn find_invalid_char(s: &str) -> Option<char> {
    s.chars().find(|&c| !c.is_alphanumeric() && c != '-' && c != '_')
}

fn start_server_for_world(world_name: &str, seed: Option<&str>) -> Result<u16, WorldStartError> {
    let world_name = world_name.replace(" ", "_");

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
    server_path.push("server/");
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

    cmd.arg("--world")
        .arg(world_path.to_str().unwrap())
        .arg("--creative")
        .arg("--local")
        .arg("--no-planets")
        .arg("--peaceful")
        .arg("--no-asteroids");

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
    add_reactable_type::<ConnectionString>(app);

    app.add_systems(
        Update,
        create_singleplayer_screen
            .run_if(in_main_menu_state(MainMenuSubState::Singleplayer))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    );
}
