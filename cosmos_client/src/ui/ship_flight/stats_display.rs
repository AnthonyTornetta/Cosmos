use bevy::{
    app::{App, Update},
    asset::AssetServer,
    color::palettes::css,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        query::{Added, With, Without},
        removal_detection::RemovedComponents,
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    text::{Text, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        FlexDirection, PositionType, Style, UiRect, Val,
    },
};
use bevy_rapier3d::dynamics::Velocity;
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::location::LocationPhysicsSet,
    structure::{
        ship::pilot::Pilot,
        systems::{energy_storage_system::EnergyStorageSystem, StructureSystems, StructureSystemsSet},
    },
};

use crate::entities::player::player_movement::PlayerMovementSet;

#[derive(Component)]
struct StatsNodes;

#[derive(Component)]
struct EnergyText;

#[derive(Component)]
struct SpeedText;

fn create_nodes(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    // q_ui_root: Query<Entity, With<UiRoot>>,
    q_became_pilot: Query<(), (With<LocalPlayer>, Added<Pilot>)>,
) {
    if !q_became_pilot.is_empty() {
        // let Ok(ui_root) = q_ui_root.get_single() else {
        //     return;
        // };

        let font = asset_server.load("fonts/PixeloidSans.ttf");

        let text_style_energy = TextStyle {
            color: css::YELLOW.into(),
            font_size: 32.0,
            font: font.clone(),
        };
        let text_style_speed = TextStyle {
            color: css::AQUAMARINE.into(),
            font_size: 32.0,
            font: font.clone(),
        };

        commands
            .spawn((
                Name::new("Ship stats ui"),
                // TargetCamera(ui_root),
                StatsNodes,
                NodeBundle {
                    style: Style {
                        padding: UiRect::all(Val::Px(10.0)),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        position_type: PositionType::Absolute,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Energy Text"),
                    EnergyText,
                    TextBundle {
                        text: Text::from_section("", text_style_energy),
                        ..Default::default()
                    },
                ));
                p.spawn((
                    Name::new("Speed Text"),
                    SpeedText,
                    TextBundle {
                        text: Text::from_section("", text_style_speed),
                        ..Default::default()
                    },
                ));
            });
    }
}

fn update_nodes(
    piloting: Query<&Pilot, With<LocalPlayer>>,
    q_piloting: Query<(&Velocity, &StructureSystems)>,
    mut q_energy_text: Query<&mut Text, (With<EnergyText>, Without<SpeedText>)>,
    mut q_speed_text: Query<&mut Text, (With<SpeedText>, Without<EnergyText>)>,

    q_energy_storage_system: Query<&EnergyStorageSystem>,
) {
    let Ok(piloting) = piloting.get_single() else {
        return;
    };

    let Ok((piloting_vel, piloting_systems)) = q_piloting.get(piloting.entity) else {
        return;
    };

    if let Ok(mut text) = q_speed_text.get_single_mut() {
        text.sections[0].value = format!("Speed: {:.1}m/s", piloting_vel.linvel.length());
    }

    if let Ok(mut text) = q_energy_text.get_single_mut() {
        if let Ok(ess) = piloting_systems.query(&q_energy_storage_system) {
            let percent = if ess.get_capacity() != 0.0 {
                ess.get_energy() / ess.get_capacity()
            } else {
                0.0
            };

            text.sections[0].value = format!("Energy {}%", (percent * 100.0).round());
        }
    }
}

fn despawn_nodes(
    mut removed_pilot: RemovedComponents<Pilot>,
    q_local_player: Query<(), With<LocalPlayer>>,
    q_stats_nodes: Query<Entity, With<StatsNodes>>,
    mut commands: Commands,
) {
    for ent in removed_pilot.read() {
        if q_local_player.contains(ent) {
            let Ok(stats_node) = q_stats_nodes.get_single() else {
                return;
            };

            commands.entity(stats_node).insert(NeedsDespawned);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            create_nodes,
            update_nodes.after(PlayerMovementSet::ProcessPlayerMovement),
            despawn_nodes,
        )
            .after(StructureSystemsSet::UpdateSystems)
            .after(LocationPhysicsSet::DoPhysics)
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    );
}
