use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use cosmos_core::structure::ship::ship_movement::ShipMovement;

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, BufferedStopAudio, CosmosAudioEmitter},
    state::game_state::GameState,
};

#[derive(Component)]
struct ThrusterSoundInstace(Handle<AudioInstance>);

fn apply_thruster_sound(
    mut query: Query<
        (
            Entity,
            &ShipMovement,
            Option<&ThrusterSoundInstace>,
            Option<&mut CosmosAudioEmitter>,
        ),
        Changed<ShipMovement>,
    >,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handle: Res<ThrusterAudioHandle>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut stop_later: ResMut<BufferedStopAudio>,
) {
    for (entity, ship_movement, thruster_sound_instance, audio_emitter) in query.iter_mut() {
        // A hacky way of determining if the thrusters are running
        let thrusters_off =
            ship_movement.movement.length_squared() + ship_movement.torque.length_squared() < 0.01 && !ship_movement.braking;

        if thrusters_off {
            if let Some(mut audio_emitter) = audio_emitter {
                if let Some(thruster_sound_instance) = thruster_sound_instance {
                    audio_emitter.remove_and_stop(&thruster_sound_instance.0, &mut audio_instances, &mut stop_later);

                    commands.entity(entity).remove::<ThrusterSoundInstace>();
                }
            }
        } else if !thrusters_off && thruster_sound_instance.is_none() {
            let playing_sound: Handle<AudioInstance> = audio.play(audio_handle.0.clone()).looped().with_volume(0.1).handle();

            let stop_tween = AudioTween::new(Duration::from_millis(400), AudioEasing::Linear);

            commands.entity(entity).insert((
                ThrusterSoundInstace(playing_sound.clone_weak()),
                CosmosAudioEmitter {
                    emissions: vec![AudioEmission {
                        instance: playing_sound,
                        max_distance: 100.0,
                        peak_volume: 0.3,
                        stop_tween,
                        ..Default::default()
                    }],
                },
            ));
        }
    }
}

#[derive(Resource)]
struct ThrusterAudioHandle(Handle<AudioSource>);

struct ThrusterSoundLoading;

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, ThrusterSoundLoading>(
        app,
        GameState::PreLoading,
        vec!["cosmos/sounds/sfx/thruster-running.ogg"],
        |mut commands, mut handles| {
            commands.insert_resource(ThrusterAudioHandle(handles.remove(0).0));
        },
    );

    app.add_systems(Update, apply_thruster_sound.run_if(in_state(GameState::Playing)));
}
