//! Client-side laser cannon system logic

use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, sync::mapping::NetworkMapping},
    physics::location::Location,
    structure::{
        ship::pilot::Pilot,
        systems::{
            missile_launcher_system::{MissileLauncherFocus, MissileLauncherPreferredFocus, MissileLauncherSystem},
            StructureSystems,
        },
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    state::game_state::GameState,
    ui::{
        ship_flight::indicators::{FocusedWaypointEntity, Indicating},
        UiRoot,
    },
};

use super::{player_interactions::HoveredSystem, sync::sync_system};

#[derive(Event)]
/// This event is fired whenever a laser cannon system is fired
pub struct MissileLauncherSystemFiredEvent(pub Entity);

#[derive(Resource)]
struct MissileLauncherFireHandles(Vec<Handle<bevy_kira_audio::prelude::AudioSource>>);

#[derive(Resource)]
struct MissileLauncherLockonGraphic(Handle<Image>);

fn apply_shooting_sound(
    query: Query<(&Location, &GlobalTransform)>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handles: Res<MissileLauncherFireHandles>,
    mut event_reader: EventReader<MissileLauncherSystemFiredEvent>,
) {
    for entity in event_reader.read() {
        let Ok((ship_location, ship_global_transform)) = query.get(entity.0) else {
            continue;
        };

        let mut location = *ship_location;
        let translation = ship_global_transform.translation();
        location.last_transform_loc = Some(ship_global_transform.translation());

        let idx = rand::random::<usize>() % audio_handles.0.len();

        let handle = audio_handles.0[idx].clone_weak();

        let playing_sound: Handle<AudioInstance> = audio.play(handle.clone_weak()).handle();

        commands.spawn((
            CosmosAudioEmitter {
                emissions: vec![AudioEmission {
                    instance: playing_sound,
                    handle,
                    ..Default::default()
                }],
            },
            DespawnOnNoEmissions,
            location,
            TransformBundle::from_transform(Transform::from_translation(translation)),
        ));
    }
}

fn focus_looking_at(
    q_systems: Query<&StructureSystems>,
    mut q_missile_focus: Query<&mut MissileLauncherPreferredFocus>,
    q_local_player: Query<&Pilot, With<LocalPlayer>>,
    q_focused: Query<Entity, With<FocusedWaypointEntity>>,
    q_indicating: Query<&Indicating>,

    mapping: Res<NetworkMapping>,
) {
    let Ok(pilot) = q_local_player.get_single() else {
        return;
    };

    let Ok(systems) = q_systems.get(pilot.entity) else {
        return;
    };

    let Ok(mut missile_focus) = systems.query_mut(&mut q_missile_focus) else {
        return;
    };

    let ent = if let Ok(focused_ent) = q_focused.get_single() {
        focused_ent
    } else {
        if missile_focus.focusing_server_entity != None {
            missile_focus.focusing_server_entity = None;
        }
        return;
    };

    let Ok(ent) = q_indicating.get(ent).map(|x| x.0) else {
        return;
    };

    let Some(server_ent) = mapping.server_from_client(&ent) else {
        if missile_focus.focusing_server_entity != None {
            missile_focus.focusing_server_entity = None;
        }

        warn!("Missing server entity for {ent:?}");

        return;
    };

    if missile_focus.focusing_server_entity != Some(server_ent) {
        missile_focus.focusing_server_entity = Some(server_ent);
    }
}

#[derive(Component)]
struct MissileFocusUi {
    left_column: Entity,
    right_column: Entity,
    top_left: Entity,
    bottom_left: Entity,
    top_right: Entity,
    bottom_right: Entity,
}

fn render_lockon_status(
    mut commands: Commands,
    lockon_graphic: Res<MissileLauncherLockonGraphic>,
    q_piloting: Query<(&HoveredSystem, &Pilot), With<LocalPlayer>>,
    q_systems: Query<&StructureSystems>,
    q_missile_focus: Query<&MissileLauncherFocus>,
    q_ui_root: Query<Entity, With<UiRoot>>,
    q_missile_focus_ui: Query<(Entity, &MissileFocusUi)>,
    mut q_style: Query<(&mut Style, &mut BackgroundColor)>,
) {
    let focus_ui = q_missile_focus_ui.get_single();

    let Ok((hovered_system, piloting)) = q_piloting.get_single() else {
        if let Ok((ent, _)) = focus_ui {
            commands.entity(ent).insert(NeedsDespawned);
        }
        return;
    };

    let Ok(systems) = q_systems.get(piloting.entity) else {
        if let Ok((ent, _)) = focus_ui {
            commands.entity(ent).insert(NeedsDespawned);
        }
        return;
    };

    let Some(missile_focus) = systems
        .try_get_activatable_system_from_activatable_index(hovered_system.hovered_system_index)
        .map(|x| q_missile_focus.get(x).ok())
        .flatten()
    else {
        if let Ok((ent, _)) = focus_ui {
            commands.entity(ent).insert(NeedsDespawned);
        }
        return;
    };

    let percentage = match missile_focus {
        MissileLauncherFocus::NotFocusing => 0.0,
        MissileLauncherFocus::Focusing {
            focusing_server_entity: _,
            focused_duration,
            complete_duration,
        } => (focused_duration.as_secs_f32() / complete_duration.as_secs_f32()).min(1.0),
    };

    let gap = (1.0 - percentage) * 100.0; // 100px is a decent number
    let color = Color::rgba(1.0, 1.0 - percentage, 1.0 - percentage, 0.7);

    if let Ok((_, focus_ui)) = focus_ui {
        update_corner_styles(&mut q_style, focus_ui.top_left, -gap, color);
        update_corner_styles(&mut q_style, focus_ui.bottom_left, gap, color);
        update_corner_styles(&mut q_style, focus_ui.top_right, -gap, color);
        update_corner_styles(&mut q_style, focus_ui.bottom_right, gap, color);

        if let Ok((mut style, _)) = q_style.get_mut(focus_ui.left_column) {
            style.margin = UiRect::right(Val::Px(gap));
        }

        if let Ok((mut style, _)) = q_style.get_mut(focus_ui.right_column) {
            style.margin = UiRect::left(Val::Px(gap));
        }
    } else {
        let Ok(ui_root) = q_ui_root.get_single() else {
            warn!("No UI root");
            return;
        };

        let mut missile_focus_ui = MissileFocusUi {
            bottom_left: Entity::PLACEHOLDER,
            bottom_right: Entity::PLACEHOLDER,
            left_column: Entity::PLACEHOLDER,
            right_column: Entity::PLACEHOLDER,
            top_left: Entity::PLACEHOLDER,
            top_right: Entity::PLACEHOLDER,
        };

        let mut ecmds = commands.spawn((
            TargetCamera(ui_root),
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            Name::new("Missile focus UI"),
        ));

        ecmds.with_children(|p| {
            missile_focus_ui.left_column = p
                .spawn((
                    Name::new("Left column"),
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            margin: UiRect::right(Val::Px(gap)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    missile_focus_ui.top_left = p
                        .spawn((
                            Name::new("Top left"),
                            ImageBundle {
                                image: UiImage {
                                    texture: lockon_graphic.0.clone_weak(),
                                    flip_x: false,
                                    flip_y: false,
                                },
                                style: Style {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    top: Val::Px(-gap),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id();

                    missile_focus_ui.bottom_left = p
                        .spawn((
                            Name::new("Bottom left"),
                            ImageBundle {
                                image: UiImage {
                                    texture: lockon_graphic.0.clone_weak(),
                                    flip_x: false,
                                    flip_y: true,
                                },
                                style: Style {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    top: Val::Px(gap),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id();
                })
                .id();

            missile_focus_ui.right_column = p
                .spawn((
                    Name::new("Right Column"),
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            margin: UiRect::left(Val::Px(gap)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    missile_focus_ui.top_right = p
                        .spawn((
                            Name::new("Top right"),
                            ImageBundle {
                                image: UiImage {
                                    texture: lockon_graphic.0.clone_weak(),
                                    flip_x: true,
                                    flip_y: false,
                                },
                                style: Style {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    top: Val::Px(-gap),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id();

                    missile_focus_ui.bottom_right = p
                        .spawn((
                            Name::new("Bottom right"),
                            ImageBundle {
                                image: UiImage {
                                    texture: lockon_graphic.0.clone_weak(),
                                    flip_x: true,
                                    flip_y: true,
                                },
                                style: Style {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    top: Val::Px(gap),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id();
                })
                .id();
        });

        ecmds.insert(missile_focus_ui);
    }
}

fn update_corner_styles(q_style: &mut Query<(&mut Style, &mut BackgroundColor)>, entity: Entity, gap: f32, color: Color) {
    let Ok((mut style, mut bg)) = q_style.get_mut(entity) else {
        return;
    };

    style.top = Val::Px(gap);
    *bg = color.into();
}

pub(super) fn register(app: &mut App) {
    sync_system::<MissileLauncherSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, MissileLauncherFireHandles>(
        app,
        GameState::PreLoading,
        vec!["cosmos/sounds/sfx/missile-launch-1.ogg", "cosmos/sounds/sfx/missile-launch-2.ogg"],
        |mut commands, handles| {
            commands.insert_resource(MissileLauncherFireHandles(
                handles.into_iter().filter(|x| x.1 == LoadState::Loaded).map(|x| x.0).collect(),
            ));
        },
    );

    load_assets::<Image, MissileLauncherLockonGraphic>(
        app,
        GameState::PreLoading,
        vec!["cosmos/images/ui/missile-lockon.png"],
        |mut commands, mut handles| {
            commands.insert_resource(MissileLauncherLockonGraphic(handles.remove(0).0));
        },
    );

    app.add_event::<MissileLauncherSystemFiredEvent>().add_systems(
        Update,
        (focus_looking_at, apply_shooting_sound, render_lockon_status).run_if(in_state(GameState::Playing)),
    );
}
