use bevy::{app::App, prelude::*};
use bevy_renet2::renet2::DisconnectReason;

use crate::{
    netty::connect::ClientDisconnectReason,
    ui::{
        components::button::{register_button, CosmosButton, ButtonEvent, ButtonStyles},
        font::DefaultFont,
        settings::SettingsMenuSet,
    },
};

use super::{in_main_menu_state, title_screen::TitleScreenSet, MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet};

fn create_disconnect_screen(
    mut commands: Commands,
    q_ui_root: Query<Entity, With<MainMenuRootUiNode>>,
    dc_reason: Option<Res<ClientDisconnectReason>>,
    default_font: Res<DefaultFont>,
) {
    let cool_blue: Color = Srgba::hex("00FFFF").unwrap().into();

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

    let Ok(main_menu_root) = q_ui_root.get_single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        p.spawn((
            Text::new("Disconnected"),
            text_style.clone(),
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
        ));

        let dc_reason = dc_reason.as_ref().map(|x| &x.0);

        info!("Disconnected: {dc_reason:?}");

        let reason_text = match dc_reason {
            None => "Unknown Reason".to_owned(),
            Some(DisconnectReason::DisconnectedByClient) => "You Quit".into(),
            Some(DisconnectReason::DisconnectedByServer) => "Disconneced by Server".into(),
            Some(DisconnectReason::PacketDeserialization(se)) => format!("Deserialization Error: {se:?}"),
            Some(DisconnectReason::PacketSerialization(se)) => format!("Serialization Error: {se:?}"),
            Some(DisconnectReason::ReceiveChannelError { channel_id, error }) => {
                format!("Recieve Channel Error (channel: {channel_id}, error: {error:?})")
            }
            Some(DisconnectReason::ReceivedInvalidChannelId(channel_id)) => format!("Got invalid channel id: {channel_id}"),
            Some(DisconnectReason::SendChannelError { channel_id, error }) => {
                format!("Send Channel Error (channel: {channel_id}, error: {error:?}")
            }
            Some(DisconnectReason::Transport) => "Unable to Establish Connection".into(),
        };

        p.spawn((
            Text::new(reason_text),
            text_style_small,
            Node {
                margin: UiRect::bottom(Val::Px(50.0)),
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
                margin: UiRect::top(Val::Px(20.0)),
                ..Default::default()
            },
            CosmosButton::<OkButtonEvent> {
                button_styles: Some(ButtonStyles {
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("OK".into(), text_style.clone(), Default::default())),
                ..Default::default()
            },
        ));
    });
}

#[derive(Default, Event, Debug)]
struct OkButtonEvent;

impl ButtonEvent for OkButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn ok_clicked(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::TitleScreen;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub(super) enum DisconnectMenuSet {
    DisconnectMenuInteractions,
}

pub(super) fn register(app: &mut App) {
    register_button::<OkButtonEvent>(app);

    app.configure_sets(
        Update,
        DisconnectMenuSet::DisconnectMenuInteractions
            .ambiguous_with(SettingsMenuSet::SettingsMenuInteractions)
            .ambiguous_with(TitleScreenSet::TitleScreenInteractions),
    );

    app.add_systems(
        Update,
        (
            create_disconnect_screen
                .run_if(in_main_menu_state(MainMenuSubState::Disconnect))
                .run_if(resource_exists_and_changed::<MainMenuSubState>)
                .in_set(MainMenuSystemSet::InitializeMenu),
            ok_clicked
                .run_if(on_event::<OkButtonEvent>)
                .run_if(in_main_menu_state(MainMenuSubState::Disconnect))
                .in_set(MainMenuSystemSet::UpdateMenu),
        )
            .in_set(DisconnectMenuSet::DisconnectMenuInteractions)
            .chain(),
    );
}
