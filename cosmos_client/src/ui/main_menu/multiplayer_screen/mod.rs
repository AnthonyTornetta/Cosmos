use std::{
    fs,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{ecs::relationship::RelatedSpawnerCommands, prelude::*, time::common_conditions::on_real_timer};
use bevy_renet::steam::steamworks::{GameServerItem, ServerListCallbacks, ServerListRequest, ServerResponse, SteamId};
use cosmos_core::state::GameState;
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
        },
        font::DefaultFont,
        main_menu::{MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet, in_main_menu_state},
        reactivity::{ReactableValue, add_reactable_type},
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

fn create_multiplayer_screen(
    mut commands: Commands,
    q_ui_root: Query<Entity, With<MainMenuRootUiNode>>,
    font: Res<DefaultFont>,
    client: Res<User>,
) {
    info!("creating multiplayer screen!@");
    let Ok(main_menu_root) = q_ui_root.single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        create_menu(p, &font, &client);
    });
}

enum ServerList {
    None,
    Some(Vec<GameServerItem>),
}

impl ServerList {
    fn add(&mut self, info: GameServerItem) {
        match self {
            Self::None => {
                *self = Self::Some(vec![info]);
            }
            Self::Some(v) => {
                v.push(info);
            }
        }
    }

    fn done(&mut self) {
        match self {
            Self::Some(v) => {
                if v.is_empty() {
                    *self = Self::None;
                }
            }
            Self::None => {}
        }
    }
}

#[derive(Component)]
struct Req(Arc<Mutex<ServerList>>);
const MULTIPLAYER_CONFIG: &str = ".multiplayer_menu_data.json";

#[derive(Default, Serialize, Deserialize, Clone)]
struct MultiplayerMenuData {
    last_connected_string: String,
}

impl MultiplayerMenuData {
    pub fn load() -> Self {
        fs::read(MULTIPLAYER_CONFIG)
            .ok()
            .and_then(|d| serde_json::from_slice::<MultiplayerMenuData>(&d).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = fs::write(MULTIPLAYER_CONFIG, serde_json::to_string_pretty(self).unwrap());
    }
}

fn create_menu(p: &mut RelatedSpawnerCommands<ChildOf>, default_font: &DefaultFont, client: &User) {
    let cool_blue = Srgba::hex("00FFFF").unwrap();

    let lan = Arc::new(Mutex::new(ServerList::Some(vec![])));
    let internet = Arc::new(Mutex::new(ServerList::Some(vec![])));

    let inner_list = lan.clone();
    let inner_list_err = lan.clone();
    let inner_list_refresh_done = lan.clone();
    client.client().matchmaking_servers().lan_server_list(
        client.client().utils().app_id(),
        ServerListCallbacks::new(
            Box::new(move |req: Arc<Mutex<ServerListRequest>>, server: i32| {
                let req = req.lock().unwrap();
                let Ok(details) = req.get_server_details(server) else {
                    error!("Bad details!");
                    return;
                };

                info!(
                    "[LAN] {} - {} ({}/{})",
                    details.server_name, details.game_description, details.players, details.max_players
                );

                inner_list.lock().unwrap().add(details);
            }),
            Box::new(move |_: Arc<Mutex<ServerListRequest>>, _: i32| {
                // let req = req.lock().unwrap();
                // let details = req.get_server_details(num);
                inner_list_err.lock().unwrap().done();
            }),
            Box::new(move |_: Arc<Mutex<ServerListRequest>>, server_res: ServerResponse| {
                info!("{server_res:?}");
                inner_list_refresh_done.lock().unwrap().done();
            }),
        ),
    );

    let inner_list = internet.clone();
    let inner_list_err = internet.clone();
    let inner_list_refresh_done = internet.clone();
    // Box<(dyn std::ops::Fn(std::sync::Arc<std::sync::Mutex<ServerListRequest>>, i32)
    client
        .client()
        .matchmaking_servers()
        .internet_server_list(
            client.client().utils().app_id(),
            &Default::default(),
            ServerListCallbacks::new(
                Box::new(move |req: Arc<Mutex<ServerListRequest>>, server: i32| {
                    let req = req.lock().unwrap();
                    let Ok(details) = req.get_server_details(server) else {
                        error!("Bad details!");
                        return;
                    };

                    info!(
                        "{} - {} ({}/{})",
                        details.server_name, details.game_description, details.players, details.max_players
                    );

                    inner_list.lock().unwrap().add(details);
                }),
                Box::new(move |_: Arc<Mutex<ServerListRequest>>, _: i32| {
                    // let req = req.lock().unwrap();
                    // let details = req.get_server_details(num);
                    inner_list_err.lock().unwrap().done();
                }),
                Box::new(move |_: Arc<Mutex<ServerListRequest>>, server_res: ServerResponse| {
                    info!("{server_res:?}");
                    inner_list_refresh_done.lock().unwrap().done();
                }),
            ),
        )
        .ok();

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
            Text::new("Multiplayer"),
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
            margin: UiRect::AUTO,
            ..Default::default()
        },
    ))
    .with_children(|p| {
        p.spawn((
            Req(lan),
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                ..Default::default()
            },
        ));

        p.spawn((
            Req(internet),
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                ..Default::default()
            },
        ));
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
            Name::new("Direct Connect"),
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
        .observe(|_: On<ButtonEvent>, mut commands: Commands| {
            let multiplayer_data = MultiplayerMenuData::load();

            commands
                .spawn((
                    Modal {
                        title: "Direct Connect".into(),
                    },
                    TextModal {
                        prompt: "Server IP or Steam ID".into(),
                        buttons: TextModalButtons::OkCancel,
                        starting_value: multiplayer_data.last_connected_string,
                        ..Default::default()
                    },
                    Name::new("Direct Connect Modal"),
                ))
                .observe(trigger_connection);
        });
    });
}

fn trigger_connection(on_complete: On<TextModalComplete>, mut state: ResMut<NextState<GameState>>, mut commands: Commands) {
    let mut data = MultiplayerMenuData::load();
    data.last_connected_string = on_complete.text.clone();
    data.save();

    let con_str = on_complete.text.replace("localhost", "127.0.0.1");
    let con_str = con_str.trim();

    info!("Trying to connect to {con_str}");

    let host_cfg = if con_str.is_empty() {
        ConnectToConfig::Ip("127.0.0.1:1337".parse().unwrap())
    } else if let Ok(hc) = con_str.parse::<u64>().map(|s_id| ConnectToConfig::SteamId(SteamId::from_raw(s_id))) {
        hc
    } else {
        let mut con_str = con_str.to_owned();
        if !con_str.contains(":") {
            con_str.push_str(":1337");
        }
        if let Ok(hc) = con_str.parse::<SocketAddr>().map(ConnectToConfig::Ip) {
            hc
        } else {
            let message = "Must be steam id or ip address";
            // em.0 = message.into();
            info!("{}", message);
            return;
        }
    };

    info!("Successful parsing of host! Starting connection process for {host_cfg:?}");

    commands.insert_resource(host_cfg);
    state.set(GameState::Connecting);
}

#[derive(Component)]
struct GameInfo {
    players: u32,
}

fn update_list_requests(q_reqs: Query<(Entity, &Req, Option<&Children>)>, mut commands: Commands, default_font: Res<DefaultFont>) {
    for (ent, req, children) in q_reqs.iter() {
        let list = req.0.lock().unwrap();
        match &*list {
            ServerList::None => {
                continue;
            }
            ServerList::Some(items) => {
                let len = items.len();
                let existing_len = if let Some(children) = children { children.len() } else { 0 };

                if existing_len == len {
                    continue;
                }

                commands.entity(ent).with_children(|p| {
                    for entry in items.iter().skip(existing_len) {
                        p.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(100.0),
                                ..Default::default()
                            },
                            GameInfo {
                                players: entry.players.max(0) as u32,
                            },
                        ))
                        .with_children(|p| {
                            let port = entry.connection_port;
                            let address = entry.addr;

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
                                move |_: On<ButtonEvent>, mut commands: Commands, mut state: ResMut<NextState<GameState>>| {
                                    let socket_addr = SocketAddr::new(IpAddr::V4(address), port);
                                    let host_cfg = ConnectToConfig::Ip(socket_addr);

                                    info!("Successful parsing of host! Starting connection process for {host_cfg:?}");

                                    commands.insert_resource(host_cfg);
                                    state.set(GameState::Connecting);
                                },
                            );

                            p.spawn((
                                Node {
                                    margin: UiRect::vertical(Val::Auto),
                                    ..Default::default()
                                },
                                Text::new(&entry.server_name),
                                TextFont {
                                    font: default_font.get(),
                                    font_size: 32.0,
                                    ..Default::default()
                                },
                            ));

                            p.spawn((
                                Node {
                                    margin: UiRect {
                                        top: Val::Auto,
                                        bottom: Val::Auto,
                                        left: Val::Px(20.0),
                                        right: Val::Px(0.0),
                                    },
                                    ..Default::default()
                                },
                                Text::new(format!("{}/{}", entry.players, entry.max_players)),
                                TextFont {
                                    font: default_font.get(),
                                    font_size: 24.0,
                                    ..Default::default()
                                },
                            ));
                        });

                        // p.spawn((
                        //     Node {
                        //         width: Val::Percent(100.0),
                        //         ..Default::default()
                        //     }
                        //     Text::new(&entry.server_name),
                        //     GameInfo {
                        //         players: entry.players.max(0) as u32,
                        //     },
                        // ));
                    }
                });
            }
        }
    }
}

fn sort_list(mut q_reqs: Query<&mut Children, With<Req>>, q_game_info: Query<&GameInfo>) {
    for mut children in q_reqs.iter_mut() {
        children.sort_by_cached_key(|x| u32::MAX - q_game_info.get(*x).expect("Missing game info ;(").players);
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<ConnectionString>(app);

    app.add_systems(
        Update,
        create_multiplayer_screen
            .run_if(in_main_menu_state(MainMenuSubState::Multiplayer))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    )
    .add_systems(
        Update,
        (update_list_requests, sort_list)
            .chain()
            .run_if(on_real_timer(Duration::from_millis(500))),
    );
}
