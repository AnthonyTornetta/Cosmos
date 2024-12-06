//! Displays debug info

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    netty::client::LocalPlayer,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::Structure,
};

use crate::{interactions::block_interactions::LookingAt, lang::Lang};

use super::{font::DefaultFont, UiRoot};

#[derive(Component, Debug, Default)]
struct FPSCounter {
    count: usize,
    delta_time: f32,
}

#[derive(Component)]
struct LookingAtText;

#[derive(Component)]
struct CoordsCounter;

#[derive(Component)]
struct ActualCoordsCounter;

fn add_text(
    mut commands: Commands,
    default_font: Res<DefaultFont>,
    q_target_camera: Query<Entity, With<UiRoot>>,
    asset_server: Res<AssetServer>,
) {
    let text_gap = 35.0;

    let target_cam = TargetCamera(q_target_camera.single());

    let font = TextFont {
        font: default_font.0.clone(),
        font_size: 32.0,
        ..Default::default()
    };

    commands.spawn((
        target_cam.clone(),
        Node {
            bottom: Val::Px(5.0),
            left: Val::Px(5.0),
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        Text::new("(x, y, z) (x, y, z)"),
        font.clone(),
        CoordsCounter,
    ));

    commands.spawn((
        target_cam.clone(),
        Node {
            bottom: Val::Px(5.0 + text_gap),
            left: Val::Px(5.0),
            position_type: PositionType::Absolute,

            ..default()
        },
        Text::new("(x, y, z)"),
        font.clone(),
        ActualCoordsCounter,
    ));

    commands.spawn((
        target_cam.clone(),
        Node {
            bottom: Val::Px(5.0 + text_gap * 2.0),
            left: Val::Px(5.0),
            position_type: PositionType::Absolute,

            ..default()
        },
        Text::new("FPS: "),
        font.clone(),
        FPSCounter::default(),
    ));

    commands
        .spawn((
            target_cam,
            Node {
                bottom: Val::Px(5.0 + text_gap * 3.0),
                left: Val::Px(5.0),
                position_type: PositionType::Absolute,

                ..default()
            },
            Text::new("Looking At: "),
            font.clone(),
        ))
        .with_children(|p| {
            p.spawn((LookingAtText, TextSpan::new("Nothing")));
        });
}

fn update_looking_at_text(
    q_looking_at: Query<&LookingAt>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    mut q_looking_at_text: Query<&mut TextSpan, With<LookingAtText>>,
    lang: Res<Lang<Block>>,
) {
    let Ok(mut text) = q_looking_at_text.get_single_mut() else {
        return;
    };

    let Ok(looking_at) = q_looking_at.get_single() else {
        return;
    };

    if let Some(looking_at) = looking_at.looking_at_any {
        let Ok(structure) = q_structure.get(looking_at.block.structure()) else {
            return;
        };

        let block = structure.block_at(looking_at.block.coords(), &blocks);
        let block_rotation = structure.block_rotation(looking_at.block.coords());

        text.as_mut().0 = format!(
            "{}: {:?}, {:?}",
            lang.get_name(block).unwrap_or(block.unlocalized_name()),
            block_rotation.face_pointing_pos_y,
            block_rotation.sub_rotation
        );
    } else {
        text.as_mut().0 = "Nothing".into();
    }
}

fn update_coords(
    query: Query<&Location, With<LocalPlayer>>,
    mut txt_coords: Query<&mut Text, (With<CoordsCounter>, Without<ActualCoordsCounter>)>,
    mut txt_coords_actual: Query<&mut Text, (With<ActualCoordsCounter>, Without<CoordsCounter>)>,
) {
    if let Ok(loc) = query.get_single() {
        for mut txt_coords in txt_coords.iter_mut() {
            txt_coords.as_mut().0 = format!("({}), ({:.1}, {:.1}, {:.1})", loc.sector(), loc.local.x, loc.local.y, loc.local.z);
        }

        for mut txt_coords_actual in txt_coords_actual.iter_mut() {
            let absolute_coords = loc.absolute_coords();
            txt_coords_actual.as_mut().0 = format!("({:.1}, {:.1}, {:.1})", absolute_coords.x, absolute_coords.y, absolute_coords.z);
        }
    }
}

fn update_fps(mut query: Query<(&mut Text, &mut FPSCounter)>, time: Res<Time>) {
    for (mut text, mut fps) in query.iter_mut() {
        fps.delta_time += time.delta_secs();

        fps.count += 1;
        if fps.delta_time >= 1.0 {
            text.as_mut().0 = format!("FPS: {}", fps.count);
            fps.count = 0;
            fps.delta_time = 0.0;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), add_text).add_systems(
        Update,
        (update_coords, update_looking_at_text, update_fps)
            .ambiguous_with_all() // none of these matter
            .run_if(in_state(GameState::Playing)),
    );
}
