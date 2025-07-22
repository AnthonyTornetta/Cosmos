use bevy::prelude::*;
use cosmos_core::{
    netty::client::LocalPlayer,
    prelude::{Asteroid, Planet, Ship, Station},
    state::GameState,
    structure::ship::pilot::Pilot,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    structure::planet::align_player::PlayerAlignment,
    ui::font::DefaultFont,
};

#[derive(Component)]
struct AlignmentHudText;

#[derive(Component)]
struct AlignmentHud;

fn create_alignment_hud(mut commands: Commands, font: Res<DefaultFont>, inputs: InputChecker) {
    commands
        .spawn((
            AlignmentHud,
            Name::new("Alignment Hud Display"),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(5.0),
                left: Val::Percent(5.0),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                AlignmentHudText,
                Text::new(""),
                TextFont {
                    font: font.get(),
                    font_size: 24.0,
                    ..Default::default()
                },
            ));
            p.spawn((
                TextFont {
                    font: font.get(),
                    font_size: 16.0,
                    ..Default::default()
                },
                Text::new(format!(
                    "Press `{}` to de-align yourself.",
                    inputs
                        .get_control(CosmosInputs::LeaveShip)
                        .map(|x| x.to_string())
                        .unwrap_or("<unbound>".to_string())
                )),
            ));
        });
}

fn update_text_to_alignment(
    mut removed_player_alignment: RemovedComponents<PlayerAlignment>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_player: Query<(Option<&PlayerAlignment>, Has<Pilot>), (With<LocalPlayer>, Or<(Changed<PlayerAlignment>, Changed<Pilot>)>)>,
    mut q_hud_node: Query<&mut Node, With<AlignmentHud>>,
    mut q_text: Query<&mut Text, With<AlignmentHudText>>,
    q_planet: Query<(), With<Planet>>,
    q_station: Query<(), With<Station>>,
    q_ship: Query<&Ship>,
    q_asteroid: Query<(), With<Asteroid>>,
) {
    if removed_player_alignment.read().any(|x| q_local_player.contains(x)) {
        if let Ok(mut node) = q_hud_node.single_mut() {
            node.display = Display::None;
        }
    }

    let Ok((alignment, pilot)) = q_player.single() else {
        return;
    };

    let Ok(mut node) = q_hud_node.single_mut() else {
        return;
    };

    if pilot {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    }

    let Some(alignment) = alignment else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };

    let aligned_to = alignment.aligned_to;
    if q_planet.contains(aligned_to) {
        // This should be obvious enough that we don't need to display this
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    }

    let Ok(mut txt) = q_text.single_mut() else {
        return;
    };

    if let Ok(_) = q_ship.get(aligned_to) {
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }

        txt.0 = format!("Aligned to Ship");
    } else if q_station.contains(aligned_to) {
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }

        txt.0 = format!("Aligned to Station");
    } else if q_asteroid.contains(aligned_to) {
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }

        txt.0 = format!("Aligned to Asteroid");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::LoadingWorld), create_alignment_hud)
        .add_systems(Update, update_text_to_alignment.run_if(in_state(GameState::Playing)));
}
