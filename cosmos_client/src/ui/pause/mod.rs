//! Pause menu

use bevy::{
    app::{App, Update},
    color::palettes::css,
    prelude::*,
};
use cosmos_core::{ecs::NeedsDespawned, state::GameState};
use renet2::RenetClient;

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    window::setup::CursorFlagsSet,
};

use super::{
    components::{
        button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
        show_cursor::ShowCursor,
    },
    settings::{NeedsSettingsAdded, SettingsCancelButtonEvent, SettingsDoneButtonEvent, SettingsMenuSet},
    OpenMenu, UiSystemSet, UiTopRoot,
};

#[derive(Resource)]
/// If this resource is present, the game is paused
pub struct Paused;

#[derive(Component)]
struct PauseMenu;

fn toggle_pause_menu(
    mut commands: Commands,
    q_open_menus: Query<(Entity, &OpenMenu)>,
    q_pause_menu: Query<Entity, With<PauseMenu>>,
    input_handler: InputChecker,
    q_ui_root: Query<Entity, With<UiTopRoot>>,
    asset_server: Res<AssetServer>,
) {
    if !input_handler.check_just_pressed(CosmosInputs::Pause) {
        return;
    }

    if !q_open_menus.is_empty() {
        if close_topmost_menus(&q_open_menus, &mut commands) {
            if let Ok(ent) = q_pause_menu.get_single() {
                commands.entity(ent).insert(Visibility::Visible);
            }
        }
        return;
    }

    if let Ok(ent) = q_pause_menu.get_single() {
        commands.entity(ent).insert(NeedsDespawned);
        commands.remove_resource::<Paused>();
        return;
    }

    let ui_root = q_ui_root.single();

    let text_style = TextStyle {
        color: css::WHITE.into(),
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap();

    let button_styles = Some(ButtonStyles {
        background_color: Srgba::hex("333333").unwrap().into(),
        hover_background_color: Srgba::hex("232323").unwrap().into(),
        press_background_color: Srgba::hex("111111").unwrap().into(),
        ..Default::default()
    });
    let style = Style {
        border: UiRect::all(Val::Px(2.0)),
        width: Val::Px(500.0),
        height: Val::Px(70.0),
        align_self: AlignSelf::Center,
        margin: UiRect::top(Val::Px(20.0)),
        ..Default::default()
    };

    commands
        .spawn((
            TargetCamera(ui_root),
            Name::new("Pause Menu"),
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_content: AlignContent::Center,
                    row_gap: Val::Px(20.0),
                    ..Default::default()
                },
                background_color: Srgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.3,
                }
                .into(),
                z_index: ZIndex::Global(100),
                ..Default::default()
            },
            PauseMenu,
            ShowCursor,
        ))
        .with_children(|p| {
            p.spawn(ButtonBundle::<ResumeButtonEvent> {
                node_bundle: NodeBundle {
                    border_color: cool_blue.into(),
                    style: style.clone(),
                    ..Default::default()
                },
                button: Button {
                    button_styles: button_styles.clone(),
                    text: Some(("RESUME".into(), text_style.clone())),
                    ..Default::default()
                },
            });

            p.spawn(ButtonBundle::<SettingsButtonEvent> {
                node_bundle: NodeBundle {
                    border_color: cool_blue.into(),
                    style: style.clone(),
                    ..Default::default()
                },
                button: Button::<SettingsButtonEvent> {
                    button_styles: button_styles.clone(),
                    text: Some(("SETTINGS".into(), text_style.clone())),
                    ..Default::default()
                },
            });

            p.spawn(ButtonBundle::<DisconnectButtonEvent> {
                node_bundle: NodeBundle {
                    border_color: cool_blue.into(),
                    style,
                    ..Default::default()
                },
                button: Button::<DisconnectButtonEvent> {
                    button_styles: button_styles.clone(),
                    text: Some(("DISCONNECT".into(), text_style.clone())),
                    ..Default::default()
                },
            });
        });
}

#[derive(Event, Debug)]
struct ResumeButtonEvent;

impl ButtonEvent for ResumeButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct SettingsButtonEvent;

impl ButtonEvent for SettingsButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct DisconnectButtonEvent;

impl ButtonEvent for DisconnectButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn close_topmost_menus(q_open_menus: &Query<(Entity, &OpenMenu)>, commands: &mut Commands) -> bool {
    let mut open = q_open_menus.iter().collect::<Vec<(Entity, &OpenMenu)>>();
    open.sort_by(|a, b| b.1.level().cmp(&a.1.level()));
    let topmost = open[0].1.level();
    for (ent, open_menu) in open.iter() {
        if open_menu.level() != topmost {
            return false;
        }
        commands.entity(*ent).insert(NeedsDespawned);
    }

    true
}

#[derive(Component)]
struct PauseMenuSettingsMenu;

fn settings_clicked(
    mut commands: Commands,
    q_target_camera: Query<Entity, With<UiTopRoot>>,
    mut q_pause_menu: Query<&mut Visibility, With<PauseMenu>>,
) {
    if let Ok(mut vis) = q_pause_menu.get_single_mut() {
        *vis = Visibility::Hidden;
    }

    let ui_root = q_target_camera.single();

    commands.spawn((
        TargetCamera(ui_root),
        Name::new("Pause Menu Settings"),
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            background_color: Srgba {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.3,
            }
            .into(),
            z_index: ZIndex::Global(101),
            ..Default::default()
        },
        OpenMenu::new(1),
        NeedsSettingsAdded,
        PauseMenuSettingsMenu,
    ));
}

fn settings_done(
    mut commands: Commands,
    q_settings_menu: Query<Entity, With<PauseMenuSettingsMenu>>,
    mut q_pause_visibility: Query<&mut Visibility, With<PauseMenu>>,
) {
    for ent in q_settings_menu.iter() {
        commands.entity(ent).insert(NeedsDespawned);
    }

    for mut vis in q_pause_visibility.iter_mut() {
        *vis = Visibility::default();
    }
}

fn disconnect_clicked(mut client: ResMut<RenetClient>) {
    client.disconnect();
}

fn resume(mut commands: Commands, q_pause_menu: Query<Entity, With<PauseMenu>>) {
    if let Ok(pause_ent) = q_pause_menu.get_single() {
        commands.entity(pause_ent).insert(NeedsDespawned);
    }

    commands.remove_resource::<Paused>();
}

pub(super) fn register(app: &mut App) {
    register_button::<ResumeButtonEvent>(app);
    register_button::<DisconnectButtonEvent>(app);
    register_button::<SettingsButtonEvent>(app);

    app.add_systems(
        Update,
        (
            toggle_pause_menu,
            settings_clicked.run_if(on_event::<SettingsButtonEvent>()).after(UiSystemSet::DoUi),
        )
            .chain()
            .run_if(in_state(GameState::Playing))
            .before(CursorFlagsSet::UpdateCursorFlags),
    )
    .add_systems(
        Update,
        (
            settings_done
                .run_if(on_event::<SettingsDoneButtonEvent>().or_else(on_event::<SettingsCancelButtonEvent>()))
                .after(SettingsMenuSet::SettingsMenuInteractions),
            resume.run_if(on_event::<ResumeButtonEvent>()).after(UiSystemSet::DoUi),
            disconnect_clicked
                .run_if(on_event::<DisconnectButtonEvent>())
                .after(UiSystemSet::DoUi),
        ),
    );
}
