use bevy::{app::App, prelude::*};
use bevy_renet2::renet2::{DisconnectReason, RenetClient};

use crate::ui::components::button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles};

use super::{in_main_menu_state, MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet};

fn create_disconnect_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    q_ui_root: Query<Entity, With<MainMenuRootUiNode>>,
    client: Option<Res<RenetClient>>,
) {
    let cool_blue: Color = Srgba::hex("00FFFF").unwrap().into();

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
    let Ok(main_menu_root) = q_ui_root.get_single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).with_children(|p| {
        p.spawn(TextBundle {
            text: Text::from_section("Disconnected", text_style.clone()),
            style: Style {
                margin: UiRect::bottom(Val::Px(20.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
            ..Default::default()
        });

        let dc_reason = client.and_then(|x| x.disconnect_reason());

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

        p.spawn(TextBundle {
            text: Text::from_section(reason_text, text_style_small),
            style: Style {
                margin: UiRect::bottom(Val::Px(50.0)),
                align_self: AlignSelf::Center,
                ..Default::default()
            },
            ..Default::default()
        });

        p.spawn(ButtonBundle::<OkButtonEvent> {
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
                    background_color: Srgba::hex("333333").unwrap().into(),
                    hover_background_color: Srgba::hex("232323").unwrap().into(),
                    press_background_color: Srgba::hex("111111").unwrap().into(),
                    ..Default::default()
                }),
                text: Some(("OK".into(), text_style.clone())),
                ..Default::default()
            },
        });
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

pub(super) fn register(app: &mut App) {
    register_button::<OkButtonEvent>(app);

    app.add_systems(
        Update,
        create_disconnect_screen
            .run_if(in_main_menu_state(MainMenuSubState::Disconnect))
            .run_if(resource_exists_and_changed::<MainMenuSubState>)
            .in_set(MainMenuSystemSet::InitializeMenu),
    )
    .add_systems(
        Update,
        ok_clicked
            .run_if(on_event::<OkButtonEvent>())
            .run_if(in_main_menu_state(MainMenuSubState::Disconnect))
            .in_set(MainMenuSystemSet::UpdateMenu),
    );
}
