//! Displays debug info

use bevy::prelude::*;
use cosmos_core::physics::location::Location;

use crate::{netty::flags::LocalPlayer, state::game_state::GameState};

#[derive(Component, Debug, Default)]
struct FPSCounter {
    count: usize,
    delta_time: f32,
}

#[derive(Component)]
struct CoordsCounter;

#[derive(Component)]
struct ActualCoordsCounter;

fn add_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = TextStyle {
        color: Color::WHITE,
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let text_gap = 35.0;

    commands.spawn((
        TextBundle {
            style: Style {
                position: UiRect {
                    bottom: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..default()
                },
                position_type: PositionType::Absolute,

                ..default()
            },
            text: Text::from_section("(x, y, z) (x, y, z)", text_style.clone()),
            ..default()
        },
        CoordsCounter,
    ));

    commands.spawn((
        TextBundle {
            style: Style {
                position: UiRect {
                    bottom: Val::Px(5.0 + text_gap),
                    left: Val::Px(5.0),
                    ..default()
                },
                position_type: PositionType::Absolute,

                ..default()
            },
            text: Text::from_section("(x, y, z)", text_style.clone()),
            ..default()
        },
        ActualCoordsCounter,
    ));

    commands.spawn((
        TextBundle {
            style: Style {
                position: UiRect {
                    bottom: Val::Px(5.0 + text_gap * 2.0),
                    left: Val::Px(5.0),
                    ..default()
                },
                position_type: PositionType::Absolute,

                ..default()
            },
            text: Text::from_section("FPS: ", text_style),
            ..default()
        },
        FPSCounter::default(),
    ));
}

fn update_coords(
    query: Query<&Location, With<LocalPlayer>>,
    mut txt_coords: Query<&mut Text, (With<CoordsCounter>, Without<ActualCoordsCounter>)>,
    mut txt_coords_actual: Query<&mut Text, (With<ActualCoordsCounter>, Without<CoordsCounter>)>,
) {
    if let Ok(loc) = query.get_single() {
        for mut txt_coords in txt_coords.iter_mut() {
            txt_coords.sections[0].value = format!(
                "({}, {}, {}), ({:.1}, {:.1}, {:.1})",
                loc.sector_x, loc.sector_y, loc.sector_z, loc.local.x, loc.local.y, loc.local.z
            );
        }

        for mut txt_coords_actual in txt_coords_actual.iter_mut() {
            let absolute_coords = loc.absolute_coords();
            txt_coords_actual.sections[0].value = format!(
                "({:.1}, {:.1}, {:.1})",
                absolute_coords.x, absolute_coords.y, absolute_coords.z
            );
        }
    }
}

fn update_fps(mut query: Query<(&mut Text, &mut FPSCounter)>, time: Res<Time>) {
    for (mut text, mut fps) in query.iter_mut() {
        fps.delta_time += time.delta_seconds();

        fps.count += 1;
        if fps.delta_time >= 1.0 {
            text.sections[0].value = format!("FPS: {}", fps.count);
            fps.count = 0;
            fps.delta_time = 0.0;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(add_text.in_schedule(OnEnter(GameState::Playing)))
        .add_systems((update_coords, update_fps).in_set(OnUpdate(GameState::Playing)));
}
