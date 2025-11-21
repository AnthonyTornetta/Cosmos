//! Pause menu

use bevy::{
    app::{App, Update},
    prelude::*,
};
use cosmos_core::{ecs::NeedsDespawned, state::GameState};
use renet::RenetClient;

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    window::setup::CursorFlagsSet,
};

use super::{
    CloseMenuMessage, CloseMethod, OpenMenu,
    components::{
        button::{ButtonMessage, ButtonStyles, CosmosButton},
        show_cursor::ShowCursor,
    },
    font::DefaultFont,
    settings::{NeedsSettingsAdded, SettingsCancelButtonMessage, SettingsDoneButtonMessage, SettingsMenuSet},
};

#[derive(Component)]
struct PauseMenu;

fn toggle_pause_menu(
    mut commands: Commands,
    mut q_open_menus: Query<(Entity, &OpenMenu, &mut Visibility)>,
    q_pause_menu: Query<Entity, With<PauseMenu>>,
    input_handler: InputChecker,
    default_font: Res<DefaultFont>,
    mut evw_close_custom_menus: MessageWriter<CloseMenuMessage>,
) {
    if !input_handler.check_just_pressed(CosmosInputs::Pause) {
        return;
    }

    if !q_open_menus.is_empty() {
        if close_topmost_menus(&mut q_open_menus, &mut commands, &mut evw_close_custom_menus)
            && let Ok(ent) = q_pause_menu.single()
        {
            commands.entity(ent).insert(Visibility::Visible);
        }
        return;
    }

    let text_style = TextFont {
        font_size: 32.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    let cool_blue = Srgba::hex("00FFFF").unwrap();

    let button_styles = Some(ButtonStyles {
        background_color: Srgba::hex("333333").unwrap().into(),
        hover_background_color: Srgba::hex("232323").unwrap().into(),
        press_background_color: Srgba::hex("111111").unwrap().into(),
        ..Default::default()
    });
    let style = Node {
        border: UiRect::all(Val::Px(2.0)),
        width: Val::Px(500.0),
        height: Val::Px(70.0),
        align_self: AlignSelf::Center,
        margin: UiRect::top(Val::Px(20.0)),
        ..Default::default()
    };

    commands
        .spawn((
            Name::new("Pause Menu"),
            Node {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Center,
                row_gap: Val::Px(20.0),
                ..Default::default()
            },
            GlobalZIndex(100),
            BackgroundColor(
                Srgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.3,
                }
                .into(),
            ),
            PauseMenu,
            OpenMenu::new(0),
            ShowCursor,
        ))
        .with_children(|p| {
            p.spawn((
                BorderColor(cool_blue.into()),
                style.clone(),
                CosmosButton {
                    button_styles: button_styles.clone(),
                    text: Some(("RESUME".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ))
            .observe(resume);

            p.spawn((
                BorderColor(cool_blue.into()),
                style.clone(),
                CosmosButton {
                    button_styles: button_styles.clone(),
                    text: Some(("SETTINGS".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ))
            .observe(settings_clicked);

            p.spawn((
                BorderColor(cool_blue.into()),
                style.clone(),
                CosmosButton {
                    button_styles: button_styles.clone(),
                    text: Some(("DISCONNECT".into(), text_style.clone(), Default::default())),
                    ..Default::default()
                },
            ))
            .observe(disconnect_clicked);
        });
}

fn close_topmost_menus(
    q_open_menus: &mut Query<(Entity, &OpenMenu, &mut Visibility)>,
    commands: &mut Commands,
    evw_close_custom_menus: &mut MessageWriter<CloseMenuMessage>,
) -> bool {
    let mut open = q_open_menus
        .iter_mut()
        .filter(|(_, open_menu, visibility)| {
            !matches!(open_menu.close_method(), CloseMethod::Visibility) || **visibility != Visibility::Hidden
        })
        .collect::<Vec<(Entity, &OpenMenu, Mut<Visibility>)>>();

    open.sort_by(|a, b| b.1.level().cmp(&a.1.level()));
    let topmost = open[0].1.level();
    for (ent, open_menu, mut visibility) in open {
        if open_menu.level() != topmost {
            return false;
        }

        match open_menu.close_method() {
            CloseMethod::Disabled => return false,
            CloseMethod::Despawn => {
                commands.entity(ent).insert(NeedsDespawned);
            }
            CloseMethod::Visibility => {
                commands
                    .entity(ent)
                    .remove::<OpenMenu>()
                    // Typically ShowCursor is used in conjunction w/ OpenMenu. Maybe these should
                    // be combined at some point?
                    .remove::<ShowCursor>();
                *visibility = Visibility::Hidden;
            }
            CloseMethod::Custom => {
                evw_close_custom_menus.write(CloseMenuMessage(ent));
            }
        }
    }

    true
}

#[derive(Component)]
struct PauseMenuSettingsMenu;

fn settings_clicked(_trigger: Trigger<ButtonMessage>, mut commands: Commands, mut q_pause_menu: Query<&mut Visibility, With<PauseMenu>>) {
    if let Ok(mut vis) = q_pause_menu.single_mut() {
        *vis = Visibility::Hidden;
    }

    commands.spawn((
        Name::new("Pause Menu Settings"),
        BackgroundColor(
            Srgba {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.3,
            }
            .into(),
        ),
        GlobalZIndex(101),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..Default::default()
        },
        OpenMenu::new(1),
        NeedsSettingsAdded,
        PauseMenuSettingsMenu,
    ));
}

fn settings_done(mut commands: Commands, q_settings_menu: Query<Entity, With<PauseMenuSettingsMenu>>) {
    for ent in q_settings_menu.iter() {
        commands.entity(ent).insert(NeedsDespawned);
    }
}

fn show_pause_if_no_settings(
    q_settings_menu: Query<(), (With<PauseMenuSettingsMenu>, Without<NeedsDespawned>)>,
    mut q_pause_visibility: Query<&mut Visibility, With<PauseMenu>>,
) {
    if q_settings_menu.is_empty() {
        for mut vis in q_pause_visibility.iter_mut() {
            *vis = Visibility::default();
        }
    }
}

fn disconnect_clicked(_trigger: Trigger<ButtonMessage>, mut client: ResMut<RenetClient>) {
    client.disconnect();
}

fn resume(_trigger: Trigger<ButtonMessage>, mut commands: Commands, q_pause_menu: Query<Entity, With<PauseMenu>>) {
    if let Ok(pause_ent) = q_pause_menu.single() {
        commands.entity(pause_ent).insert(NeedsDespawned);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// When the `Escape` key is pressed to open the pause menu, this set will be called
pub enum CloseMenusSet {
    /// This set is when close any menus open will be closed when `Escape` is pressed.
    CloseMenus,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, CloseMenusSet::CloseMenus);

    app.add_systems(
        Update,
        (toggle_pause_menu.in_set(CloseMenusSet::CloseMenus),)
            .chain()
            .run_if(in_state(GameState::Playing))
            .before(CursorFlagsSet::UpdateCursorFlags),
    )
    .add_systems(
        Update,
        (
            settings_done
                .run_if(on_message::<SettingsDoneButtonMessage>.or(on_message::<SettingsCancelButtonMessage>))
                .after(SettingsMenuSet::SettingsMenuInteractions),
            show_pause_if_no_settings,
        )
            .chain(),
    );
}
