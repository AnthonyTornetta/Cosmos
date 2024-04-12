//! Client-side laser cannon system logic

use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    netty::sync::mapping::NetworkMapping,
    physics::location::Location,
    structure::{
        ship::pilot::Pilot,
        systems::{
            missile_launcher_system::{MissileLauncherPreferredFocus, MissileLauncherSystem},
            StructureSystems,
        },
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
    ui::ship_flight::indicators::{ClosestWaypoint, FocusedWaypointEntity, Indicating},
};

use super::sync::sync_system;

#[derive(Event)]
/// This event is fired whenever a laser cannon system is fired
pub struct MissileLauncherSystemFiredEvent(pub Entity);

#[derive(Resource)]
struct MissileLauncherFireHandles(Vec<Handle<bevy_kira_audio::prelude::AudioSource>>);

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
    closest_waypoint: Res<ClosestWaypoint>,
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
    } else if let Some(closest_waypoint) = closest_waypoint.0 {
        closest_waypoint
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

struct MissileLauncherLoadingFlag;

pub(super) fn register(app: &mut App) {
    sync_system::<MissileLauncherSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, MissileLauncherLoadingFlag>(
        app,
        GameState::PreLoading,
        vec!["cosmos/sounds/sfx/missile-launch-1.ogg", "cosmos/sounds/sfx/missile-launch-2.ogg"],
        |mut commands, handles| {
            commands.insert_resource(MissileLauncherFireHandles(
                handles.into_iter().filter(|x| x.1 == LoadState::Loaded).map(|x| x.0).collect(),
            ));
        },
    );

    app.add_event::<MissileLauncherSystemFiredEvent>().add_systems(
        Update,
        (focus_looking_at, apply_shooting_sound).run_if(in_state(GameState::Playing)),
    );
}
