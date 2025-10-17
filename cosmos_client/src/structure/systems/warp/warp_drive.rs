use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioEasing, AudioInstance, AudioSource, AudioTween};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    state::GameState,
    structure::systems::warp::warp_drive::{WarpDriveInitiating, WarpDriveSystem},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
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
    );
}
