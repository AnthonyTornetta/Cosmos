use bevy::{app::App, core_pipeline::bloom::BloomSettings, hierarchy::DespawnRecursiveExt, prelude::*, render::camera::Camera};
use bevy_kira_audio::prelude::AudioReceiver;

use crate::{
    lang::Lang,
    state::game_state::GameState,
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
            scollable_container::{ScrollBox, ScrollBundle},
            slider::{Slider, SliderBundle},
            text_input::{InputType, TextInput, TextInputBundle},
            window::{GuiWindow, WindowBundle},
            Disabled,
        },
        reactivity::{add_reactable_type, BindValue, BindValues, ReactableFields, ReactableValue},
        UiSystemSet,
    },
};

use super::{
    components::{show_cursor::ShowCursor, text_input::InputValue},
    UiRoot,
};

mod menu_panorama;

#[derive(Component)]
struct DespawnOnSwitchState;

#[derive(Component)]
struct MainMenuCamera;

#[derive(Component)]
struct MainMenuRootUiNode(f32);

fn despawn_all_main_menu_ents(mut commands: Commands, q_main_menu_entities: Query<Entity, With<DespawnOnSwitchState>>) {
    for e in q_main_menu_entities.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn spin_camera(mut q_main_menu_camera: Query<&mut Transform, With<MainMenuCamera>>, time: Res<Time>) {
    for mut trans in q_main_menu_camera.iter_mut() {
        trans.rotation *= Quat::from_axis_angle(Vec3::Y, time.delta_seconds() / 30.0);
    }
}

#[derive(Default, Event, Debug)]
struct ConnectButtonEvent;

impl ButtonEvent for ConnectButtonEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

fn create_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
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
        color: Color::WHITE,
        font_size: 256.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let cam_id = commands
        .spawn((
            DespawnOnSwitchState,
            MainMenuCamera,
            Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..Default::default()
                },
                transform: Transform::default(),
                projection: Projection::from(PerspectiveProjection {
                    fov: (90.0 / 180.0) * std::f32::consts::PI,
                    ..default()
                }),
                ..default()
            },
            BloomSettings { ..Default::default() },
            Name::new("Main Menu Camera"),
            UiRoot,
            AudioReceiver,
            ShowCursor,
        ))
        .id();

    commands
        .spawn((
            MainMenuRootUiNode(0.0),
            TargetCamera(cam_id),
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_content: AlignContent::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|p| {
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
                    border_color: Color::AQUAMARINE.into(),
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
                    border_color: Color::AQUAMARINE.into(),
                    background_color: Color::hex("333333").unwrap().into(),
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
        });
}

fn fade_in_background(mut q_root_node: Query<(&mut MainMenuRootUiNode, &mut BackgroundColor)>, time: Res<Time>) {
    for (mut root, mut bg) in q_root_node.iter_mut() {
        const MIN_A: f32 = 0.6;

        let alpha_now = (1.0 / (6.0 * root.0) + MIN_A).min(1.0);

        bg.0 = Color::rgba(bg.0.r(), bg.0.g(), bg.0.b(), alpha_now);

        root.0 += time.delta_seconds();
    }
}

pub(super) fn register(app: &mut App) {
    menu_panorama::register(app);

    register_button::<ConnectButtonEvent>(app);

    app.add_systems(OnEnter(GameState::MainMenu), create_main_menu);

    app.add_systems(Update, (spin_camera, fade_in_background).run_if(in_state(GameState::MainMenu)));

    app.add_systems(OnExit(GameState::MainMenu), despawn_all_main_menu_ents);
}
