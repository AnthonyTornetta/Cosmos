//! Handles the client-side logic of dying + respawning

use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    entities::{
        health::Dead,
        player::respawn::{RequestRespawnMessage, RespawnMessage},
    },
    netty::{client::LocalPlayer, sync::events::client_event::NettyMessageWriter},
    physics::location::{Location, SetPosition},
};
use renet::RenetClient;

use crate::ui::{
    CloseMenuMessage, CloseMethod, OpenMenu, UiSystemSet,
    components::{
        button::{ButtonEvent, ButtonStyles, CosmosButton},
        show_cursor::ShowCursor,
    },
    font::DefaultFont,
};

#[derive(Component)]
struct DeathUi;

fn display_death_ui(
    mut commands: Commands,
    mut q_open_menus: Query<(Entity, &OpenMenu, &mut Visibility, &mut Node)>,
    q_added_death: Query<(), (Added<Dead>, With<LocalPlayer>)>,
    font: Res<DefaultFont>,
    mut evw_close_custom_menus: MessageWriter<CloseMenuMessage>,
) {
    if q_added_death.is_empty() {
        return;
    }

    for (ent, open_menu, mut visibility, mut node) in q_open_menus.iter_mut() {
        match open_menu.close_method() {
            CloseMethod::Disabled => continue,
            CloseMethod::Despawn => {
                commands.entity(ent).insert(NeedsDespawned);
            }
            CloseMethod::Display => {
                commands.entity(ent).remove::<OpenMenu>().remove::<ShowCursor>();
                node.display = Display::None;
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

    commands
        .spawn((
            OpenMenu::with_close_method(0, CloseMethod::Disabled),
            ShowCursor,
            Name::new("Death Ui"),
            DeathUi,
            BackgroundColor(
                Srgba {
                    red: 0.0,
                    blue: 0.0,
                    green: 0.0,
                    alpha: 0.8,
                }
                .into(),
            ),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            let btn_node = Node {
                width: Val::Px(350.0),
                padding: UiRect {
                    left: Val::Px(20.0),
                    right: Val::Px(20.0),
                    top: Val::Px(16.0),
                    bottom: Val::Px(16.0),
                },
                margin: UiRect::all(Val::Px(15.0)),
                ..Default::default()
            };

            let btn_font = TextFont {
                font_size: 36.0,
                font: font.0.clone(),
                ..Default::default()
            };

            let color = TextColor(Color::WHITE);

            let btn_styles = Some(ButtonStyles {
                background_color: Srgba::hex("#555").unwrap().into(),
                hover_background_color: Srgba::hex("#777").unwrap().into(),
                press_background_color: css::AQUA.into(),
                foreground_color: Color::WHITE,
                press_foreground_color: Color::WHITE,
                hover_foreground_color: Color::WHITE,
            });

            p.spawn((
                Text::new("You Died ;("),
                Node {
                    margin: UiRect::bottom(Val::Px(100.0)),
                    ..Default::default()
                },
                TextFont {
                    font_size: 48.0,
                    font: font.0.clone(),
                    ..Default::default()
                },
            ));

            p.spawn((
                btn_node.clone(),
                CosmosButton {
                    text: Some(("Respawn".into(), btn_font.clone(), color)),
                    button_styles: btn_styles.clone(),
                    ..Default::default()
                },
            ))
            .observe(respawn_clicked);

            p.spawn((
                btn_node,
                CosmosButton {
                    text: Some(("Quit".into(), btn_font, color)),
                    button_styles: btn_styles,
                    ..Default::default()
                },
            ))
            .observe(title_screen_clicked);
        });
}

fn on_not_dead(
    mut commands: Commands,
    q_respawn_ui: Query<Entity, With<DeathUi>>,
    mut removed_components: RemovedComponents<Dead>,
    q_local_player: Query<(), With<LocalPlayer>>,
) {
    for c in removed_components.read() {
        if q_local_player.contains(c)
            && let Ok(ent) = q_respawn_ui.single()
        {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

fn on_respawn(
    mut commands: Commands,
    mut evr_respawn: MessageReader<RespawnMessage>,
    mut q_local_player: Query<(Entity, &mut Location, &mut Transform), With<LocalPlayer>>,
) {
    for ev in evr_respawn.read() {
        let Ok((entity, mut loc, mut trans)) = q_local_player.single_mut() else {
            continue;
        };

        *loc = ev.location;
        trans.rotation = ev.rotation;

        // not removing parent in place, since we're setting the transform's rotation aboslutely
        commands.entity(entity).remove::<ChildOf>().insert(SetPosition::Transform);
    }
}

fn respawn_clicked(_trigger: On<ButtonEvent>, mut nevw_respawn: NettyMessageWriter<RequestRespawnMessage>) {
    nevw_respawn.write_default();
}

fn title_screen_clicked(_trigger: On<ButtonEvent>, mut client: ResMut<RenetClient>) {
    client.disconnect();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (display_death_ui.before(UiSystemSet::PreDoUi), on_not_dead).chain())
        .add_systems(FixedUpdate, on_respawn.in_set(FixedUpdateSet::Main));
}
