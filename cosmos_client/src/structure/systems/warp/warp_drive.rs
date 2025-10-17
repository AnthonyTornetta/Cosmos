use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioEasing, AudioInstance, AudioSource, AudioTween};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::client::LocalPlayer,
    state::GameState,
    structure::{
        ship::pilot::Pilot,
        systems::warp::warp_drive::{WarpDriveInitiating, WarpDriveSystem},
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    rendering::MainCamera,
    structure::systems::sync::sync_system,
};

#[derive(Resource)]
struct WarpSound(Handle<AudioSource>);

fn play_warp_sound(
    mut commands: Commands,
    q_started_warping: Query<Entity, Added<WarpDriveInitiating>>,
    audio: Res<Audio>,
    audio_handle: Res<WarpSound>,
) {
    for entity in q_started_warping.iter() {
        let playing_sound: Handle<AudioInstance> = audio.play(audio_handle.0.clone_weak()).with_volume(0.0).handle();

        let stop_tween = AudioTween::new(Duration::from_millis(100), AudioEasing::Linear);

        commands.entity(entity).with_child((
            Transform::default(),
            DespawnOnNoEmissions,
            // We do NOT want to despawn with structure - so omit the `DespawnWithStructure` here.
            // This way, if the ship warps away this sfx remains where the ship was.
            CosmosAudioEmitter {
                emissions: vec![AudioEmission {
                    instance: playing_sound,
                    max_distance: 1000.0,
                    peak_volume: 0.3 * 5.0,
                    stop_tween,
                    handle: audio_handle.0.clone_weak(),
                }],
            },
        ));
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
    } else {
        if cam.scale != Vec3::ONE {
            cam.scale = Vec3::ONE;
        }
    }
}

pub(super) fn register(app: &mut App) {
    sync_system::<WarpDriveSystem>(app);

    load_assets::<AudioSource, WarpSound, 1>(
        app,
        GameState::Loading,
        ["cosmos/sounds/sfx/warp-jump.ogg"],
        |mut cmds, [(sound, _)]| {
            cmds.insert_resource(WarpSound(sound));
        },
    );

    app.add_systems(
        FixedUpdate,
        play_warp_sound.run_if(in_state(GameState::Playing)).in_set(FixedUpdateSet::Main),
    )
    .add_systems(Update, play_warp_animation.run_if(in_state(GameState::Playing)));
}
