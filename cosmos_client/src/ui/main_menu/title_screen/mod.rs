use bevy::{app::App, prelude::*};

use crate::{
    state::game_state::GameState,
    ui::components::{
        button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
        text_input::{InputType, TextInput, TextInputBundle},
    },
};

use super::{super::components::text_input::InputValue, in_main_menu_state, MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet};

fn create_main_menu(mut commands: Commands, asset_server: Res<AssetServer>, q_ui_root: Query<Entity, With<MainMenuRootUiNode>>) {
    let cool_blue = Color::hex("00FFFF").unwrap();

    let text_style = TextStyle {
        color: Color::WHITE,
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };
    let text_style_small = TextStyle {
        color: Color::WHITE,
        font_size: 24.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };
    let text_style_large = TextStyle {
        color: cool_blue,
        font_size: 256.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let Ok(main_menu_root) = q_ui_root.get_single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        p.spawn(TextBundle {
            text: Text::from_section("COSMOS", text_style_large),
            style: Style {
                margin: UiRect::bottom(Val::Px(200.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
            ..Default::default()
        });

        p.spawn(ButtonBundle::<ConnectButtonEvent> {
            node_bundle: NodeBundle {
                border_color: cool_blue.into(),
                style: Style {
                    border: UiRect::all(Val::Px(2.0)),
                    width: Val::Px(500.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    ..Default::default()
                },
                ..Default::default()
            },
            button: Button {
                button_styles: Some(ButtonStyles {
                    background_color: Color::hex("333333").unwrap(),
                    hover_background_color: Color::hex("232323").unwrap(),
                    press_background_color: Color::hex("111111").unwrap(),
                    ..Default::default()
                }),
                text: Some(("Connect".into(), text_style.clone())),
                ..Default::default()
            },
        });

        p.spawn(TextInputBundle {
            text_input: TextInput {
                style: text_style_small.clone(),
                input_type: InputType::Text { max_length: None },
                ..Default::default()
            },
            value: InputValue::new("localhost"),
            node_bundle: NodeBundle {
                border_color: Color::hex("555555").unwrap().into(),
                background_color: Color::hex("111111").unwrap().into(),
                style: Style {
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
                ..Default::default()
            },
            ..Default::default()
        });

        p.spawn(ButtonBundle::<SettingsButtonEvent> {
            node_bundle: NodeBundle {
                border_color: cool_blue.into(),
                style: Style {
                    border: UiRect::all(Val::Px(2.0)),
                    width: Val::Px(500.0),
                    height: Val::Px(70.0),
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
            button: Button {
                button_styles: Some(ButtonStyles {
                    background_color: Color::hex("333333").unwrap(),
                    hover_background_color: Color::hex("232323").unwrap(),
                    press_background_color: Color::hex("111111").unwrap(),
                    ..Default::default()
                }),
                text: Some(("Settings".into(), text_style.clone())),
                ..Default::default()
            },
        });
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

fn goto_settings(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::Settings;
}

fn trigger_connection(mut state: ResMut<NextState<GameState>>) {
    state.set(GameState::Connecting);
}

pub(super) fn register(app: &mut App) {
    register_button::<ConnectButtonEvent>(app);
    register_button::<SettingsButtonEvent>(app);

    app.add_systems(
        Update,
        create_main_menu
            .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    )
    .add_systems(
        Update,
        goto_settings
            .run_if(on_event::<SettingsButtonEvent>())
            .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
            .in_set(MainMenuSystemSet::UpdateMenu),
    )
    .add_systems(
        Update,
        trigger_connection
            .run_if(on_event::<ConnectButtonEvent>())
            .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
            .in_set(MainMenuSystemSet::UpdateMenu),
    );
}
