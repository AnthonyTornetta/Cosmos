//! Displays debug info

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    netty::client::LocalPlayer,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

use crate::{interactions::block_interactions::LookingAt, lang::Lang, state::game_state::GameState};

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
                bottom: Val::Px(5.0),
                left: Val::Px(5.0),
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
                bottom: Val::Px(5.0 + text_gap),
                left: Val::Px(5.0),
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
                bottom: Val::Px(5.0 + text_gap * 2.0),
                left: Val::Px(5.0),
                position_type: PositionType::Absolute,

                ..default()
            },
            text: Text::from_section("FPS: ", text_style.clone()),
            ..default()
        },
        FPSCounter::default(),
    ));

    commands.spawn((
        TextBundle {
            style: Style {
                bottom: Val::Px(5.0 + text_gap * 3.0),
                left: Val::Px(5.0),
                position_type: PositionType::Absolute,

                ..default()
            },
            text: Text::from_sections(vec![
                TextSection::new("Looking At: ", text_style.clone()),
                TextSection::new("Nothing", text_style.clone()),
            ]),
            ..default()
        },
        LookingAtText,
    ));
}

fn update_looking_at_text(
    q_looking_at: Query<&LookingAt>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    mut q_looking_at_text: Query<&mut Text, With<LookingAtText>>,
    lang: Res<Lang<Block>>,
) {
    let Ok(mut text) = q_looking_at_text.get_single_mut() else {
        return;
    };

    let Ok(looking_at) = q_looking_at.get_single() else {
        return;
    };

    if let Some((structure_ent, structure_block)) = looking_at.looking_at_block {
        let Ok(structure) = q_structure.get(structure_ent) else {
            return;
        };

        let block = structure.block_at(structure_block.coords(), &blocks);
        let block_rotation = structure.block_rotation(structure_block.coords());

        text.sections[1].value = format!(
            "{}: {:?}, {:?}",
            lang.get_name(block).unwrap_or(block.unlocalized_name()),
            block_rotation.block_up,
            block_rotation.sub_rotation
        );
    } else {
        text.sections[1].value = "Nothing".into();
    }
}

fn update_coords(
    query: Query<&Location, With<LocalPlayer>>,
    mut txt_coords: Query<&mut Text, (With<CoordsCounter>, Without<ActualCoordsCounter>)>,
    mut txt_coords_actual: Query<&mut Text, (With<ActualCoordsCounter>, Without<CoordsCounter>)>,
) {
    if let Ok(loc) = query.get_single() {
        for mut txt_coords in txt_coords.iter_mut() {
            txt_coords.sections[0].value = format!("({}), ({:.1}, {:.1}, {:.1})", loc.sector(), loc.local.x, loc.local.y, loc.local.z);
        }

        for mut txt_coords_actual in txt_coords_actual.iter_mut() {
            let absolute_coords = loc.absolute_coords();
            txt_coords_actual.sections[0].value = format!("({:.1}, {:.1}, {:.1})", absolute_coords.x, absolute_coords.y, absolute_coords.z);
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
    app.add_systems(OnEnter(GameState::Playing), add_text).add_systems(
        Update,
        (update_coords, update_looking_at_text, update_fps).run_if(in_state(GameState::Playing)),
    );
}
