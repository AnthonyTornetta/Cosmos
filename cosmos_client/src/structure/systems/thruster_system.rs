//! Client-side thruster system logic

use std::time::Duration;

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    state::GameState,
    structure::{
        ship::ship_movement::{ShipMovement, ShipMovementSet},
        systems::thruster_system::ThrusterSystem,
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, AudioSet, BufferedStopAudio, CosmosAudioEmitter, volume::Volume},
};

use super::sync::sync_system;

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
        let thrusters_off = ship_movement.movement.length_squared() + ship_movement.torque.length_squared() < 0.01
            && !ship_movement.braking
            && !ship_movement.match_speed;

        if thrusters_off {
            if let Some(mut audio_emitter) = audio_emitter
                && let Some(thruster_sound_instance) = thruster_sound_instance
            {
                audio_emitter.remove_and_stop(&thruster_sound_instance.0, &mut audio_instances, &mut stop_later);

                commands.entity(entity).remove::<ThrusterSoundInstace>();
            }
        } else if !thrusters_off && thruster_sound_instance.is_none() {
            let playing_sound: Handle<AudioInstance> = audio.play(audio_handle.0.clone()).with_volume(0.0).looped().handle();

            let stop_tween = AudioTween::new(Duration::from_millis(400), AudioEasing::Linear);

            commands.entity(entity).insert((
                ThrusterSoundInstace(playing_sound.clone()),
                CosmosAudioEmitter::with_emissions(vec![AudioEmission {
                    instance: playing_sound,
                    max_distance: 100.0,
                    peak_volume: Volume::new_unbound(1.5),
                    stop_tween,
                    handle: audio_handle.0.clone(),
                }]),
            ));
        }
    }
}

#[derive(Resource)]
struct ThrusterAudioHandle(Handle<bevy_kira_audio::prelude::AudioSource>);

struct ThrusterSoundLoading;

pub(super) fn register(app: &mut App) {
    sync_system::<ThrusterSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, ThrusterSoundLoading, 1>(
        app,
        GameState::PreLoading,
        ["cosmos/sounds/sfx/thruster-running.ogg"],
        |mut commands, [sound]| {
            commands.insert_resource(ThrusterAudioHandle(sound.0));
        },
    );

    app.add_systems(
        Update,
        apply_thruster_sound
            .in_set(AudioSet::CreateSounds)
            .after(ShipMovementSet::RemoveShipMovement)
            .run_if(in_state(GameState::Playing)),
    );
}
