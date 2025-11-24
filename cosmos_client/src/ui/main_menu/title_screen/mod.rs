use std::net::SocketAddr;

use bevy::{
    app::{App, AppExit},
    prelude::*,
};
use bevy_renet::steam::steamworks::SteamId;
use cosmos_core::state::GameState;

use crate::{
    netty::connect::ConnectToConfig,
    ui::{
        components::{
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            text_input::{InputType, TextInput},
        },
        font::DefaultFont,
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
        settings::SettingsMenuSet,
    },
};

use super::{
    super::components::text_input::InputValue, MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet,
    disconnect_screen::DisconnectMenuSet, in_main_menu_state,
};

fn create_main_menu(mut commands: Commands, default_font: Res<DefaultFont>, q_ui_root: Query<Entity, With<MainMenuRootUiNode>>) {
    let cool_blue = Srgba::hex("00FFFF").unwrap().into();

    let text_style = TextFont {
        font_size: 32.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let text_blue = TextColor(cool_blue);
    let text_style_large = TextFont {
        font_size: 256.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let Ok(main_menu_root) = q_ui_root.single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        p.spawn((
            Text::new("COSMOS"),
            text_style_large,
            text_blue,
            Node {
                margin: UiRect::bottom(Val::Px(50.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
        ));

        p.spawn((
            BorderColor::all(cool_blue),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                margin: UiRect::all(Val::Px(10.0)),
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
                text: Some(("Singleplayer".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ))
        .observe(goto_state(MainMenuSubState::Singleplayer));

        p.spawn((
            BorderColor::all(cool_blue),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                margin: UiRect::all(Val::Px(10.0)),
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
                text: Some(("Multiplayer".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ))
        .observe(goto_state(MainMenuSubState::Multiplayer));

        p.spawn((
            BorderColor::all(cool_blue),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                align_self: AlignSelf::Center,
                margin: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            CosmosButton {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("Settings".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ))
        .observe(goto_state(MainMenuSubState::Settings));

        p.spawn((
            BorderColor::all(cool_blue),
            Node {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(500.0),
                height: Val::Px(70.0),
                align_self: AlignSelf::Center,
                margin: UiRect::all(Val::Px(10.0)),
                ..Default::default()
            },
            CosmosButton {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("Quit".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ))
        .observe(quit_game);
    });
}

fn goto_state(s: MainMenuSubState) -> impl Fn(On<ButtonEvent>, ResMut<MainMenuSubState>) {
    move |_on: On<ButtonEvent>, mut mms: ResMut<MainMenuSubState>| {
        *mms = s;
    }
}

fn quit_game(_trigger: On<ButtonEvent>, mut evw_app_exit: MessageWriter<AppExit>) {
    info!("Triggering quit game!");
    evw_app_exit.write(AppExit::Success);
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub(super) enum TitleScreenSet {
    TitleScreenInteractions,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        TitleScreenSet::TitleScreenInteractions
            .ambiguous_with(DisconnectMenuSet::DisconnectMenuInteractions)
            .ambiguous_with(SettingsMenuSet::SettingsMenuInteractions),
    );

    app.add_systems(
        Update,
        (create_main_menu
            .run_if(in_main_menu_state(MainMenuSubState::TitleScreen))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),)
            .in_set(TitleScreenSet::TitleScreenInteractions)
            .chain(),
    );
}
