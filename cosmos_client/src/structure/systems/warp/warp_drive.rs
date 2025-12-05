use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioEasing, AudioInstance, AudioSource, AudioTween};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::client::LocalPlayer,
    state::GameState,
    structure::{
        ship::pilot::Pilot,
        systems::warp::warp_drive::{WarpCancelledMessage, WarpDriveInitiating, WarpDriveSystem},
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, BufferedStopAudio, CosmosAudioEmitter, DespawnOnNoEmissions, volume::Volume},
    rendering::MainCamera,
    structure::systems::sync::sync_system,
};

#[derive(Resource)]
struct WarpSound {
    warp: Handle<AudioSource>,
    shutdown: Handle<AudioSource>,
}

#[derive(Component)]
struct WarpSoundMarker(Handle<AudioInstance>);

fn play_warp_sound(
    mut commands: Commands,
    q_started_warping: Query<Entity, Added<WarpDriveInitiating>>,
    audio: Res<Audio>,
    audio_handle: Res<WarpSound>,
) {
    for entity in q_started_warping.iter() {
        let playing_sound: Handle<AudioInstance> = audio.play(audio_handle.warp.clone()).with_volume(0.0).handle();

        let stop_tween = AudioTween::new(Duration::from_millis(100), AudioEasing::Linear);

        commands.entity(entity).with_child((
            Transform::default(),
            DespawnOnNoEmissions,
            WarpSoundMarker(playing_sound.clone()),
            // We do NOT want to despawn with structure - so omit the `DespawnWithStructure` here.
            // This way, if the ship warps away this sfx remains where the ship was.
            CosmosAudioEmitter {
                emissions: vec![AudioEmission {
                    instance: playing_sound,
                    max_distance: 1000.0,
                    peak_volume: Volume::default(),
                    stop_tween,
                    handle: audio_handle.warp.clone(),
                }],
            },
        ));
    }
}

fn on_shutdown_warp(
    mut nevr_shutdown_warp: MessageReader<WarpCancelledMessage>,
    mut q_warp_sound: Query<(&mut CosmosAudioEmitter, &WarpSoundMarker)>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut stop_later: ResMut<BufferedStopAudio>,
    q_children: Query<&Children>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handle: Res<WarpSound>,
) {
    for e in nevr_shutdown_warp.read() {
        let Ok(children) = q_children.get(e.structure_entity) else {
            continue;
        };

        for &child in children {
            if let Ok((mut audio_emitter, sound_marker)) = q_warp_sound.get_mut(child) {
                audio_emitter.remove_and_stop(&sound_marker.0, &mut audio_instances, &mut stop_later);
                commands.entity(child).remove::<WarpSoundMarker>();

                let shutdown_sound: Handle<AudioInstance> = audio.play(audio_handle.shutdown.clone()).with_volume(0.0).handle();

                let stop_tween = AudioTween::new(Duration::from_millis(100), AudioEasing::Linear);
                audio_emitter.add_emission(AudioEmission {
                    instance: shutdown_sound,
                    max_distance: 1000.0,
                    peak_volume: Volume::new(0.2),
                    stop_tween,
                    handle: audio_handle.shutdown.clone(),
                });

                break;
            }
        }
    }
}

fn play_warp_animation(
    q_warping: Query<(&Pilot, &WarpDriveInitiating)>,
    q_local_player: Query<(), With<LocalPlayer>>,
    mut q_main_cam: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut cam) = q_main_cam.single_mut() else {
        error!("Missing main cam!");
        return;
    };

    if let Some((_, warp_drive_initating)) = q_warping.iter().find(|(p, _)| q_local_player.contains(p.entity)) {
        const MAX_ZOOM: f32 = 0.8;
        let amt = MAX_ZOOM - (MAX_ZOOM * (warp_drive_initating.charge.powf(2.0) / warp_drive_initating.max_charge.powf(2.0))).min(MAX_ZOOM)
            + (1.0 - MAX_ZOOM);
        cam.scale.x = amt;
        cam.scale.y = amt;
    } else if cam.scale != Vec3::ONE {
        cam.scale = Vec3::ONE;
    }
}

pub(super) fn register(app: &mut App) {
    sync_system::<WarpDriveSystem>(app);

    load_assets::<AudioSource, WarpSound, 2>(
        app,
        GameState::Loading,
        ["cosmos/sounds/sfx/warp-jump.ogg", "cosmos/sounds/sfx/warp-drive-shutdown.ogg"],
        |mut cmds, [(warp, _), (shutdown, _)]| {
            cmds.insert_resource(WarpSound { warp, shutdown });
        },
    );

    app.add_systems(
        FixedUpdate,
        (play_warp_sound, on_shutdown_warp)
            .run_if(in_state(GameState::Playing))
            .in_set(FixedUpdateSet::Main),
    )
    .add_systems(Update, play_warp_animation.run_if(in_state(GameState::Playing)));
}
