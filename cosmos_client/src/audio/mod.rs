//! Handles spacial audio a bit more nicely than bevy_kira_audio does.
//!
//! Heavily based on https://github.com/NiklasEi/bevy_kira_audio/blob/main/src/spacial.rs
//!
//! Note that this logic does rely on the `AudioReceiver` defined in kira's implementation.

use bevy::{prelude::*, utils::HashMap};
use bevy_kira_audio::{prelude::*, AudioSystemSet};

#[derive(Debug)]
/// Contains information for a specific audio emission.
///
/// Do make sure the [`instance`] is unique to this [`AudioEmission`], as this directly modifies that instance to get the 3d effect.
///
/// This must be attached to a [`CosmosAudioEmitter`] to do anything.
pub struct AudioEmission {
    /// The instance of audio to play
    pub instance: Handle<AudioInstance>,
    /// The maximum distance you can hear this sound from - defaults to 100.0
    pub max_distance: f32,
    /// The max volume this sound will play at - defaults to 1.0
    pub peak_volume: f64,
}

impl Default for AudioEmission {
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            peak_volume: 1.0,
            instance: Default::default(),
        }
    }
}

#[derive(Default, Debug, Component)]
/// Contains a bunch of audio instances to output
///
/// This must be put onto an entity with a transform to do anything
pub struct CosmosAudioEmitter {
    /// The audio sources to output
    pub emissions: Vec<AudioEmission>,
}

#[derive(Default, Bundle)]
/// A bundle containing a [`CosmosAudioEmitter`] and a [`TransformBundle`]
pub struct AudioEmitterBundle {
    /// The thing responsible for emitting the audio
    pub emitter: CosmosAudioEmitter,
    /// The transform this source is at
    pub transform: TransformBundle,
}

fn run_spacial_audio(
    receiver: Query<&GlobalTransform, With<AudioReceiver>>,
    emitters: Query<(&GlobalTransform, &CosmosAudioEmitter)>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    if let Ok(receiver_transform) = receiver.get_single() {
        for (emitter_transform, emitter) in emitters.iter() {
            let sound_path = emitter_transform.translation() - receiver_transform.translation();

            let right_ear_angle = receiver_transform.right().angle_between(sound_path);
            let panning = ((right_ear_angle.cos() + 1.0) / 2.0) as f64;

            for emission in emitter.emissions.iter() {
                if let Some(instance) = audio_instances.get_mut(&emission.instance) {
                    let volume = emission.peak_volume * (1.0 - sound_path.length() / emission.max_distance).clamp(0., 1.).powi(2) as f64;

                    instance.set_volume(volume, AudioTween::default());
                    instance.set_panning(panning, AudioTween::default());
                }
            }
        }
    } else {
        warn!("There are {} audio receivers - should always be 1.", receiver.iter().count());
    }
}

fn cleanup_stopped_spacial_instances(mut emitters: Query<&mut CosmosAudioEmitter>, instances: Res<Assets<AudioInstance>>) {
    for mut emitter in emitters.iter_mut() {
        let handles = &mut emitter.emissions;

        handles.retain(|emission| {
            if let Some(instance) = instances.get(&emission.instance) {
                instance.state() != PlaybackState::Stopped
            } else {
                true
            }
        });
    }
}

#[derive(Debug, Default, Resource)]
struct AttachedAudioSources(HashMap<Entity, Vec<Handle<AudioInstance>>>);

fn monitor_attached_audio_sources(
    mut attached_audio_sources: ResMut<AttachedAudioSources>,
    query: Query<(Entity, &CosmosAudioEmitter), Changed<CosmosAudioEmitter>>,
) {
    for (entity, audio_emitter) in query.iter() {
        attached_audio_sources
            .0
            .insert(entity, audio_emitter.emissions.iter().map(|x| x.instance.clone_weak()).collect());
    }
}

fn cleanup_despawning_audio_sources(
    mut removed_emitters: RemovedComponents<CosmosAudioEmitter>,
    mut attached_audio_sources: ResMut<AttachedAudioSources>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    for entity in removed_emitters.iter() {
        if let Some(instances) = attached_audio_sources.0.remove(&entity) {
            for audio_instance in instances {
                if let Some(mut ai) = audio_instances.remove(&audio_instance) {
                    ai.stop(AudioTween::default());
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreUpdate, cleanup_stopped_spacial_instances.in_set(AudioSystemSet::InstanceCleanup))
        .add_systems(Update, (monitor_attached_audio_sources, cleanup_despawning_audio_sources).chain())
        .add_systems(PostUpdate, run_spacial_audio)
        .init_resource::<AttachedAudioSources>();
}
