use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::{
        health::Dead,
        player::respawn::{RequestRespawnEvent, RespawnEvent},
    },
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    physics::location::{Location, LocationPhysicsSet, SetPosition},
};
use renet2::RenetClient;

use crate::ui::{
    components::{
        button::{register_button, ButtonEvent, ButtonStyles, CosmosButton},
        show_cursor::ShowCursor,
    },
    font::DefaultFont,
    CloseMethod, OpenMenu, UiSystemSet,
};

#[derive(Component)]
struct DeathUi;

#[derive(Event, Debug)]
struct RespawnBtnClicked;
impl ButtonEvent for RespawnBtnClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}
#[derive(Event, Debug)]
struct TitleScreenBtnClicked;
impl ButtonEvent for TitleScreenBtnClicked {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn display_death_ui(
    mut commands: Commands,
    mut q_open_menus: Query<(Entity, &OpenMenu, &mut Visibility)>,
    q_added_death: Query<(), (Added<Dead>, With<LocalPlayer>)>,
    font: Res<DefaultFont>,
) {
    if q_added_death.is_empty() {
        return;
    }

    for (ent, open_menu, mut visibility) in q_open_menus.iter_mut() {
        match open_menu.close_method() {
            CloseMethod::Disabled => continue,
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
                    alpha: 0.2,
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
                width: Val::Px(250.0),
                padding: UiRect {
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                    top: Val::Px(12.0),
                    bottom: Val::Px(12.0),
                },
                margin: UiRect::all(Val::Px(25.0)),
                ..Default::default()
            };

            let btn_font = TextFont {
                font_size: 24.0,
                font: font.0.clone_weak(),
                ..Default::default()
            };

            let color = TextColor(Color::WHITE);

            let btn_styles = Some(ButtonStyles {
                background_color: Srgba::hex("#333").unwrap().into(),
                hover_background_color: Srgba::hex("#555").unwrap().into(),
                press_background_color: css::AQUA.into(),
                foreground_color: Color::WHITE,
                press_foreground_color: Color::WHITE,
                hover_foreground_color: Color::WHITE,
                ..Default::default()
            });

            p.spawn((
                Text::new("You Died ;("),
                Node {
                    margin: UiRect::bottom(Val::Px(100.0)),
                    ..Default::default()
                },
                TextFont {
                    font_size: 48.0,
                    font: font.0.clone_weak(),
                    ..Default::default()
                },
            ));

            p.spawn((
                btn_node.clone(),
                CosmosButton::<RespawnBtnClicked> {
                    text: Some(("Respawn".into(), btn_font.clone(), color.clone())),
                    button_styles: btn_styles.clone(),
                    ..Default::default()
                },
            ));

            p.spawn((
                btn_node,
                CosmosButton::<TitleScreenBtnClicked> {
                    text: Some(("Quit".into(), btn_font, color)),
                    button_styles: btn_styles,
                    ..Default::default()
                },
            ));
        });
}

fn on_not_dead(
    mut commands: Commands,
    q_respawn_ui: Query<Entity, With<DeathUi>>,
    mut removed_components: RemovedComponents<Dead>,
    q_local_player: Query<(), With<LocalPlayer>>,
) {
    for c in removed_components.read() {
        if q_local_player.contains(c) {
            if let Ok(ent) = q_respawn_ui.get_single() {
                commands.entity(ent).insert(NeedsDespawned);
            }
        }
    }
}

fn on_respawn(
    mut commands: Commands,
    mut evr_respawn: EventReader<RespawnEvent>,
    mut q_local_player: Query<(Entity, &mut Location, &mut Transform), With<LocalPlayer>>,
) {
    for ev in evr_respawn.read() {
        let Ok((entity, mut loc, mut trans)) = q_local_player.get_single_mut() else {
            continue;
        };

        *loc = ev.location;
        trans.rotation = ev.rotation;

        // not removing parent in place, since we're setting the transform's rotation aboslutely
        commands.entity(entity).remove_parent().insert(SetPosition::Transform);
    }
}

fn respawn_clicked(mut nevw_respawn: NettyEventWriter<RequestRespawnEvent>) {
    nevw_respawn.send_default();
}

fn title_screen_clicked(mut client: ResMut<RenetClient>) {
    client.disconnect();
}

pub(super) fn register(app: &mut App) {
    register_button::<RespawnBtnClicked>(app);
    register_button::<TitleScreenBtnClicked>(app);

    app.add_systems(
        Update,
        (
            display_death_ui.before(UiSystemSet::PreDoUi),
            on_not_dead,
            title_screen_clicked
                .after(UiSystemSet::FinishUi)
                .run_if(on_event::<TitleScreenBtnClicked>),
            respawn_clicked.after(UiSystemSet::FinishUi).run_if(on_event::<RespawnBtnClicked>),
            on_respawn.before(LocationPhysicsSet::DoPhysics),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    );
}
