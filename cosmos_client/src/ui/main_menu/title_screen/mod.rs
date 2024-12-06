use std::net::ToSocketAddrs;

use bevy::{
    app::{App, AppExit},
    prelude::*,
};
use cosmos_core::state::GameState;

use crate::{
    netty::connect::HostConfig,
    ui::{
        components::{
            button::{register_button, Button, ButtonEvent, ButtonStyles},
            text_input::{InputType, TextInput},
        },
        font::DefaultFont,
        reactivity::{add_reactable_type, BindValue, BindValues, ReactableFields, ReactableValue},
        settings::SettingsMenuSet,
    },
};

use super::{
    super::components::text_input::InputValue, disconnect_screen::DisconnectMenuSet, in_main_menu_state, MainMenuRootUiNode,
    MainMenuSubState, MainMenuSystemSet,
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

fn create_main_menu(
    mut commands: Commands,
    default_font: Res<DefaultFont>,
    asset_server: Res<AssetServer>,
    q_ui_root: Query<Entity, With<MainMenuRootUiNode>>,
) {
    let cool_blue = Srgba::hex("00FFFF").unwrap().into();

    let text_style = TextFont {
        font_size: 32.0,
        font: default_font.0.clone(),
        ..Default::default()
    };
    let text_style_small = TextFont {
        font_size: 24.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let text_blue = TextColor(cool_blue);
    let text_style_large = TextFont {
        font_size: 256.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let Ok(main_menu_root) = q_ui_root.get_single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        p.spawn((
            Text::new("COSMOS"),
            text_style_large,
            Node {
                margin: UiRect::bottom(Val::Px(200.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
        ));

        p.spawn((
            BorderColor(cool_blue),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
            Button::<ConnectButtonEvent> {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("Connect".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ));

        let vars_entity = p.spawn((ConnectionString("localhost".into()), ErrorMessage::default())).id();

        p.spawn((
            BindValues::single(BindValue::<ConnectionString>::new(vars_entity, ReactableFields::Value)),
            text_style_small.clone(),
            TextInput {
                input_type: InputType::Text { max_length: None },
                ..Default::default()
            },
            InputValue::new("localhost"),
            BorderColor(Srgba::hex("555555").unwrap().into()),
            BackgroundColor(Srgba::hex("111111").unwrap().into()),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(45.0),
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

        p.spawn((
            BorderColor(cool_blue.into()),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                align_self: AlignSelf::Center,
                margin: UiRect::top(Val::Px(20.0)),
                ..Default::default()
            },
            Button::<SettingsButtonEvent> {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("Settings".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ));

        p.spawn((
            BorderColor(cool_blue.into()),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                align_self: AlignSelf::Center,
                margin: UiRect::top(Val::Px(20.0)),
                ..Default::default()
            },
            Button::<QuitButtonEvent> {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("Quit".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ));
    });
}

#[derive(Default, Event, Debug)]
struct ConnectButtonEvent;

impl ButtonEvent for ConnectButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Default, Event, Debug)]
struct SettingsButtonEvent;

impl ButtonEvent for SettingsButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Default, Event, Debug)]
struct QuitButtonEvent;

impl ButtonEvent for QuitButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn goto_settings(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::Settings;
}

fn trigger_connection(
    mut q_vars: Query<(&ConnectionString, &mut ErrorMessage)>,
    mut state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    let Ok((connection_string, mut em)) = q_vars.get_single_mut() else {
        return;
    };

    info!("Parsing connection string: {connection_string:?}");

    let mut split = connection_string.0.split(":");

    let Some(host_name) = split.next() else {
        em.0 = "Must specify host".to_owned();
        return;
    };

    let port = split.next();
    let excess = split.next();

    if excess.is_some() {
        em.0 = "Cannot have multiple colons in address".to_owned();
        return;
    }

    let port = if let Some(port) = port {
        if let Ok(port) = port.parse::<u16>() {
            port
        } else {
            em.0 = "Invalid port".to_owned();
            return;
        }
    } else {
        1337
    };

    let host_name = host_name.trim();
    if host_name.is_empty() {
        em.0 = "Must specify host".to_owned();
    }

    if format!("{host_name}:{port}").to_socket_addrs().is_err() {
        em.0 = "Invalid host".to_owned();
        return;
    }

    commands.insert_resource(HostConfig {
        host_name: host_name.into(),
        port,
    });
    state.set(GameState::Connecting);
}

fn quit_game(mut evw_app_exit: EventWriter<AppExit>) {
    evw_app_exit.send(AppExit::Success);
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub(super) enum TitleScreenSet {
    TitleScreenInteractions,
}

pub(super) fn register(app: &mut App) {
    register_button::<ConnectButtonEvent>(app);
    register_button::<SettingsButtonEvent>(app);
    register_button::<QuitButtonEvent>(app);

    add_reactable_type::<ConnectionString>(app);

    app.configure_sets(
        Update,
        TitleScreenSet::TitleScreenInteractions
            .ambiguous_with(DisconnectMenuSet::DisconnectMenuInteractions)
            .ambiguous_with(SettingsMenuSet::SettingsMenuInteractions),
    );

    app.add_systems(
        Update,
        (
            create_main_menu
                .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
                .run_if(resource_exists_and_changed::<MainMenuSubState>)
                .in_set(MainMenuSystemSet::InitializeMenu),
            goto_settings
                .run_if(on_event::<SettingsButtonEvent>)
                .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
                .in_set(MainMenuSystemSet::UpdateMenu),
            trigger_connection
                .run_if(in_state(GameState::MainMenu))
                .run_if(on_event::<ConnectButtonEvent>)
                .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
                .in_set(MainMenuSystemSet::UpdateMenu),
            quit_game
                .run_if(on_event::<QuitButtonEvent>)
                .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
                .in_set(MainMenuSystemSet::UpdateMenu),
        )
            .in_set(TitleScreenSet::TitleScreenInteractions)
            .chain(),
    );
}
