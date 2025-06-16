//! Client-side laser cannon system logic

use bevy::{asset::LoadState, color::palettes::css, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet},
    physics::location::{Location, LocationPhysicsSet},
    state::GameState,
    structure::{
        ship::pilot::Pilot,
        systems::{
            StructureSystems,
            missile_launcher_system::{MissileLauncherFocus, MissileLauncherPreferredFocus, MissileLauncherSystem, MissileSystemFailure},
        },
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    ui::{
        message::{HudMessage, HudMessages},
        ship_flight::indicators::{FocusedWaypointEntity, Indicating},
    },
};

use super::{
    player_interactions::{HoveredSystem, SystemUsageSet},
    sync::sync_system,
};

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

        let location = *ship_location;
        let translation = ship_global_transform.translation();

        let idx = rand::random::<u64>() as usize % audio_handles.0.len();

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
            Transform::from_translation(translation),
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
    let Ok(pilot) = q_local_player.single() else {
        return;
    };

    let Ok(systems) = q_systems.get(pilot.entity) else {
        return;
    };

    let Ok(mut missile_focus) = systems.query_mut(&mut q_missile_focus) else {
        return;
    };

    let ent = if let Ok(focused_ent) = q_focused.single() {
        focused_ent
    } else {
        if missile_focus.focusing_server_entity.is_some() {
            missile_focus.focusing_server_entity = None;
        }
        return;
    };

    let Ok(ent) = q_indicating.get(ent).map(|x| x.0) else {
        return;
    };

    let Some(server_ent) = mapping.server_from_client(&ent) else {
        // This can happen if the entity is dead on the client, but we're getting out of date data from the server

        if missile_focus.focusing_server_entity.is_some() {
            missile_focus.focusing_server_entity = None;
        }

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
    q_missile_focus_ui: Query<(Entity, &MissileFocusUi)>,
    mut q_style: Query<(&mut Node, &mut ImageNode)>,
    mut q_column_style: Query<&mut Node, Without<ImageNode>>,
) {
    let focus_ui = q_missile_focus_ui.single();

    let Ok((hovered_system, piloting)) = q_piloting.single() else {
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
        .and_then(|x| q_missile_focus.get(x).ok())
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
    let color: Color = Srgba {
        red: 1.0,
        green: 1.0 - percentage,
        blue: 1.0 - percentage,
        alpha: 0.7,
    }
    .into();

    if let Ok((_, focus_ui)) = focus_ui {
        update_corner_styles(&mut q_style, focus_ui.top_left, -gap, color);
        update_corner_styles(&mut q_style, focus_ui.bottom_left, gap, color);
        update_corner_styles(&mut q_style, focus_ui.top_right, -gap, color);
        update_corner_styles(&mut q_style, focus_ui.bottom_right, gap, color);

        if let Ok(mut style) = q_column_style.get_mut(focus_ui.left_column) {
            style.margin = UiRect::right(Val::Px(gap));
        }

        if let Ok(mut style) = q_column_style.get_mut(focus_ui.right_column) {
            style.margin = UiRect::left(Val::Px(gap));
        }
    } else {
        let mut missile_focus_ui = MissileFocusUi {
            bottom_left: Entity::PLACEHOLDER,
            bottom_right: Entity::PLACEHOLDER,
            left_column: Entity::PLACEHOLDER,
            right_column: Entity::PLACEHOLDER,
            top_left: Entity::PLACEHOLDER,
            top_right: Entity::PLACEHOLDER,
        };

        let mut ecmds = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            Name::new("Missile focus UI"),
        ));

        ecmds.with_children(|p| {
            missile_focus_ui.left_column = p
                .spawn((
                    Name::new("Left column"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::right(Val::Px(gap)),
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    missile_focus_ui.top_left = p
                        .spawn((
                            Name::new("Top left"),
                            ImageNode {
                                image: lockon_graphic.0.clone_weak(),
                                flip_x: false,
                                flip_y: false,
                                ..Default::default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                top: Val::Px(-gap),
                                ..Default::default()
                            },
                        ))
                        .id();

                    missile_focus_ui.bottom_left = p
                        .spawn((
                            Name::new("Bottom left"),
                            ImageNode {
                                image: lockon_graphic.0.clone_weak(),
                                flip_x: false,
                                flip_y: true,
                                ..Default::default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                top: Val::Px(gap),
                                ..Default::default()
                            },
                        ))
                        .id();
                })
                .id();

            missile_focus_ui.right_column = p
                .spawn((
                    Name::new("Right Column"),
                    Node {
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::left(Val::Px(gap)),
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    missile_focus_ui.top_right = p
                        .spawn((
                            Name::new("Top right"),
                            ImageNode {
                                image: lockon_graphic.0.clone_weak(),
                                flip_x: true,
                                flip_y: false,
                                ..Default::default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                top: Val::Px(-gap),
                                ..Default::default()
                            },
                        ))
                        .id();

                    missile_focus_ui.bottom_right = p
                        .spawn((
                            Name::new("Bottom right"),
                            ImageNode {
                                image: lockon_graphic.0.clone_weak(),
                                flip_x: true,
                                flip_y: true,
                                ..Default::default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                top: Val::Px(gap),
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

fn update_corner_styles(q_style: &mut Query<(&mut Node, &mut ImageNode)>, entity: Entity, gap: f32, color: Color) {
    let Ok((mut style, mut img)) = q_style.get_mut(entity) else {
        return;
    };

    style.top = Val::Px(gap);
    img.color = color;
}

fn listen_for_errors_firing(mut evr_missile_launcher_failer: EventReader<MissileSystemFailure>, mut hud_messages: ResMut<HudMessages>) {
    for ev in evr_missile_launcher_failer.read() {
        match ev {
            MissileSystemFailure::NoAmmo => {
                hud_messages.display_message(HudMessage::with_colored_string("No missiles to fire!", css::RED.into()));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    sync_system::<MissileLauncherSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, MissileLauncherFireHandles, 2>(
        app,
        GameState::PreLoading,
        ["cosmos/sounds/sfx/missile-launch-1.ogg", "cosmos/sounds/sfx/missile-launch-2.ogg"],
        |mut commands, handles| {
            commands.insert_resource(MissileLauncherFireHandles(
                handles
                    .into_iter()
                    .filter(|x| matches!(x.1, LoadState::Loaded))
                    .map(|x| x.0)
                    .collect(),
            ));
        },
    );

    load_assets::<Image, MissileLauncherLockonGraphic, 1>(
        app,
        GameState::PreLoading,
        ["cosmos/images/ui/missile-lockon.png"],
        |mut commands, [image]| {
            commands.insert_resource(MissileLauncherLockonGraphic(image.0));
        },
    );

    app.add_event::<MissileLauncherSystemFiredEvent>().add_systems(
        Update,
        (
            focus_looking_at,
            apply_shooting_sound,
            render_lockon_status,
            listen_for_errors_firing,
        )
            .chain()
            .after(SystemUsageSet::ChangeSystemBeingUsed)
            .in_set(NetworkingSystemsSet::Between)
            .after(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing)),
    );
}
