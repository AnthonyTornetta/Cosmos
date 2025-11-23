use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use bevy::{
    ecs::relationship::{RelatedSpawner, RelatedSpawnerCommands},
    prelude::*,
};
use bevy_renet::steam::steamworks::{ServerListCallbacks, ServerListRequest, ServerResponse, SteamId};
use cosmos_core::state::GameState;

use crate::{
    netty::{connect::ConnectToConfig, steam::User},
    ui::{
        components::{
            button::{ButtonMessage, ButtonStyles, CosmosButton},
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

    p.spawn((
        BorderColor::all(cool_blue),
        Node {
            border: UiRect::all(Val::Px(2.0)),
            width: Val::Px(500.0),
            height: Val::Px(70.0),
            align_self: AlignSelf::Center,
            ..Default::default()
        },
        CosmosButton {
            button_styles: Some(ButtonStyles {
                background_color: Srgba::hex("333333").unwrap().into(),
                hover_background_color: Srgba::hex("232323").unwrap().into(),
                press_background_color: Srgba::hex("111111").unwrap().into(),
                ..Default::default()
            }),
            text: Some(("Connect".into(), text_style.clone(), Default::default())),
            ..Default::default()
        },
    ))
    .observe(trigger_connection);

    client.client().matchmaking_servers().lan_server_list(
        client.client().utils().app_id(),
        ServerListCallbacks::new(
            Box::new(|req: Arc<Mutex<ServerListRequest>>, server: i32| {
                let req = req.lock().unwrap();
                let Ok(details) = req.get_server_details(server) else {
                    error!("Bad details!");
                    return;
                };

                info!(
                    "[LAN] {} - {} ({}/{})",
                    details.server_name, details.game_description, details.players, details.max_players
                );
            }),
            Box::new(|_: Arc<Mutex<ServerListRequest>>, _: i32| {
                // let req = req.lock().unwrap();
                // let details = req.get_server_details(num);
            }),
            Box::new(|_: Arc<Mutex<ServerListRequest>>, server_res: ServerResponse| {
                info!("{server_res:?}");
            }),
        ),
    );

    // Box<(dyn std::ops::Fn(std::sync::Arc<std::sync::Mutex<ServerListRequest>>, i32)
    let requests = client.client().matchmaking_servers().internet_server_list(
        client.client().utils().app_id(),
        &Default::default(),
        ServerListCallbacks::new(
            Box::new(|req: Arc<Mutex<ServerListRequest>>, server: i32| {
                let req = req.lock().unwrap();
                let Ok(details) = req.get_server_details(server) else {
                    error!("Bad details!");
                    return;
                };

                info!(
                    "{} - {} ({}/{})",
                    details.server_name, details.game_description, details.players, details.max_players
                );
            }),
            Box::new(|_: Arc<Mutex<ServerListRequest>>, _: i32| {
                // let req = req.lock().unwrap();
                // let details = req.get_server_details(num);
            }),
            Box::new(|_: Arc<Mutex<ServerListRequest>>, server_res: ServerResponse| {
                info!("{server_res:?}");
            }),
        ),
    );

    /*
        * |res| {
    match res {
                Ok(list) => {
                    info!("{list:?}");
                }
                Err(e) => {
                    error!("{e:?}");
                }

            }, |f| {

                }, |r| {

                })         ;
    */
    let vars_entity = p.spawn((ConnectionString("localhost".into()), ErrorMessage::default())).id();

    p.spawn((
        BindValues::single(BindValue::<ConnectionString>::new(vars_entity, ReactableFields::Value)),
        text_style_small.clone(),
        TextInput {
            input_type: InputType::Text { max_length: None },
            ..Default::default()
        },
        InputValue::new("localhost"),
        BorderColor::all(Srgba::hex("555555").unwrap()),
        BackgroundColor(Srgba::hex("111111").unwrap().into()),
        Node {
            border: UiRect::all(Val::Px(2.0)),
            width: Val::Px(500.0),
            min_height: Val::Px(45.0),
            align_self: AlignSelf::Center,
            margin: UiRect::top(Val::Px(20.0)),
            padding: UiRect {
                top: Val::Px(4.0),
                bottom: Val::Px(4.0),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn trigger_connection(
    _trigger: On<ButtonMessage>,
    mut q_vars: Query<(&ConnectionString, &mut ErrorMessage)>,
    mut state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    let Ok((connection_string, mut em)) = q_vars.single_mut() else {
        return;
    };

    let con_str = connection_string.0.replace("localhost", "127.0.0.1");
    let con_str = con_str.trim();

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
            em.0 = "Must be steam id or ip address".into();
            info!("{}", em.0);
            return;
        }
    };

    commands.insert_resource(host_cfg);
    state.set(GameState::Connecting);
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<ConnectionString>(app);

    app.add_systems(
        Update,
        create_multiplayer_screen
            .run_if(in_main_menu_state(MainMenuSubState::Multiplayer))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    );
}
