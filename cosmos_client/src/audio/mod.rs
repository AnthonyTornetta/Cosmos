//! Handles spacial audio a bit more nicely than bevy_kira_audio does.
//!
//! Heavily based on https://github.com/NiklasEi/bevy_kira_audio/blob/main/src/spacial.rs
//!
//! Note that this logic does rely on the `AudioReceiver` defined in kira's implementation.

use bevy::{prelude::*, utils::hashbrown::HashMap};
use bevy_kira_audio::{prelude::*, AudioSystemSet};

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
    /// Tween used when the sound is removed from the audio emitter - Default will immediately cut it off
    pub stop_tween: AudioTween,
    /// A weak-cloned handle that is being played. This is to prevent too many of the same audio source blowing people's ears out
    pub handle: Handle<AudioSource>,
}

impl Default for AudioEmission {
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            peak_volume: 1.0,
            instance: Default::default(),
            handle: Default::default(),
            stop_tween: Default::default(),
        }
    }
}

#[derive(Default, Component)]
/// Contains a bunch of audio instances to output
///
/// This must be put onto an entity with a transform to do anything
pub struct CosmosAudioEmitter {
    /// The audio sources to output
    pub emissions: Vec<AudioEmission>,
}

#[derive(Component, Default, Debug)]
/// This flag will despawn the entity when there are no more audio emissions occuring
pub struct DespawnOnNoEmissions;

#[derive(Default, Resource, Deref, DerefMut)]
/// Fixes the issue of sounds failing to stop because of a full command queue.
///
/// Simply add your audio instance into this, and it will *eventually* be stopped, once the queue has enough space.
///
/// This generally happens within the same frame, do don't be too worried about delays.
pub struct BufferedStopAudio(Vec<(AudioInstance, AudioTween)>);

impl CosmosAudioEmitter {
    /// Constructs an audio emitter with the given emissions
    pub fn with_emissions(emissions: Vec<AudioEmission>) -> Self {
        Self { emissions }
    }

    /// Removes the audio handle from this emitter and returns the emission if one is present.
    ///
    /// This does not stop playing the audio, it is up to you to handle it.
    pub fn remove(&mut self, handle: &Handle<AudioInstance>) -> Option<AudioEmission> {
        if let Some(idx) = self.emissions.iter().position(|x| x.instance == *handle) {
            Some(self.emissions.swap_remove(idx))
        } else {
            None
        }
    }

    /// Removes and stops the audio handle from this emitter and returns the emission if one is present.
    ///
    /// `audio_instances` is just ResMut<Assets<AudioInstance>>
    pub fn remove_and_stop(
        &mut self,
        handle: &Handle<AudioInstance>,
        audio_instances: &mut Assets<AudioInstance>,
        stop_later: &mut BufferedStopAudio,
    ) -> Option<AudioEmission> {
        if let Some(removed) = self.remove(handle) {
            if let Some(instance) = audio_instances.remove(&removed.instance) {
                stop_later.push((instance, removed.stop_tween.clone()));
            } else {
                warn!("NO INSTANCE FOUND!")
            }

            Some(removed)
        } else {
            None
        }
    }
}

#[derive(Default, Bundle)]
/// A bundle containing a [`CosmosAudioEmitter`] and a [`TransformBundle`]
pub struct AudioEmitterBundle {
    /// The thing responsible for emitting the audio
    pub emitter: CosmosAudioEmitter,
    /// The transform this source is at
    pub transform: TransformBundle,
}

/// Prevents too many sounds of the same type creating an awful sound
// const MAX_AUDIO_SOURCES_PLAYING_SAME_SOUND: usize = 4;

fn run_spacial_audio(
    receiver: Query<&GlobalTransform, With<AudioReceiver>>,
    emitters: Query<(&GlobalTransform, &CosmosAudioEmitter)>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    let Ok(receiver_transform) = receiver.get_single() else {
        return;
    };

    // let mut num_audios_of_same_source: HashMap<AssetId<AudioSource>, usize> = HashMap::default();

    for (emitter_transform, emitter) in emitters.iter() {
        let sound_path = emitter_transform.translation() - receiver_transform.translation();

        let mut right_ear_angle = receiver_transform.right().angle_between(sound_path);
        // This happens if you're facing a direction that is exactly in line with whatever it is for some reason.
        if right_ear_angle.is_nan() {
            right_ear_angle = 0.0;
        }

        let panning = ((right_ear_angle.cos() + 1.0) / 2.0) as f64;

        for emission in emitter.emissions.iter() {
            let Some(instance) = audio_instances.get_mut(&emission.instance) else {
                continue;
            };

            let volume = emission.peak_volume * (1.0 - sound_path.length() / emission.max_distance).clamp(0., 1.).powi(2) as f64;

            // if let Some(amt) = num_audios_of_same_source.get_mut(&emission.handle.id()) {
            //     *amt += 1;

            //     if *amt == MAX_AUDIO_SOURCES_PLAYING_SAME_SOUND {
            //         instance.set_volume(emission.peak_volume, AudioTween::default());
            //         // 0.5 = equal left + right panning
            //         instance.set_panning(0.5, AudioTween::default());
            //         continue;
            //     } else if *amt > MAX_AUDIO_SOURCES_PLAYING_SAME_SOUND {
            //         instance.set_volume(emission.peak_volume, AudioTween::default());
            //         instance.set_panning(0.5, AudioTween::default());
            //         continue;
            //     }
            // } else {
            //     num_audios_of_same_source.insert(emission.handle.id(), 1);
            // }

            instance.set_volume(volume, AudioTween::default());
            instance.set_panning(panning, AudioTween::default());
        }
    }
}

type AttachedAudioSourcesType = Vec<(Handle<AudioInstance>, AudioTween)>;

#[derive(Default, Resource)]
struct AttachedAudioSources(HashMap<Entity, AttachedAudioSourcesType>);

fn monitor_attached_audio_sources(
    mut attached_audio_sources: ResMut<AttachedAudioSources>,
    query: Query<(Entity, &CosmosAudioEmitter), Changed<CosmosAudioEmitter>>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    for (entity, audio_emitter) in query.iter() {
        let cur_items = attached_audio_sources.0.remove(&entity).unwrap_or(vec![]);

        let new_items = audio_emitter
            .emissions
            .iter()
            .map(|x| (x.instance.clone_weak(), x.stop_tween.clone()))
            .collect::<AttachedAudioSourcesType>();

        let mut remove_vec = vec![];

        for current_item in cur_items {
            if !new_items.iter().any(|x| x.0 == current_item.0) {
                remove_vec.push(current_item);
            }
        }

        attached_audio_sources.0.insert(entity, new_items);

        // Stop any removed audio emissions
        for (audio_instance, tween) in remove_vec {
            if let Some(mut ai) = audio_instances.remove(&audio_instance) {
                if ai.state() != PlaybackState::Stopped {
                    ai.stop(tween);
                }
            }
        }
    }
}

fn cleanup_stopped_spacial_instances(
    mut emitters: Query<(Entity, &mut CosmosAudioEmitter, Option<&DespawnOnNoEmissions>)>,
    instances: Res<Assets<AudioInstance>>,
    mut commands: Commands,
) {
    for (entity, mut emitter, despawn_when_empty) in emitters.iter_mut() {
        let handles = &mut emitter.emissions;

        handles.retain(|emission| {
            if let Some(instance) = instances.get(&emission.instance) {
                instance.state() != PlaybackState::Stopped
            } else {
                true
            }
        });

        if handles.is_empty() && despawn_when_empty.is_some() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn cleanup_despawning_audio_sources(
    mut removed_emitters: RemovedComponents<CosmosAudioEmitter>,
    mut attached_audio_sources: ResMut<AttachedAudioSources>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    for entity in removed_emitters.read() {
        if let Some(instances) = attached_audio_sources.0.remove(&entity) {
            for (audio_instance, tween) in instances {
                if let Some(mut ai) = audio_instances.remove(&audio_instance) {
                    ai.stop(tween);
                }
            }
        }
    }
}

fn stop_audio_sources(mut stop_later: ResMut<BufferedStopAudio>) {
    let mut old_stop_later = BufferedStopAudio(Vec::with_capacity(stop_later.capacity()));

    std::mem::swap(&mut old_stop_later, &mut stop_later);

    for (mut instance, tween) in old_stop_later.0 {
        if instance.stop(tween.clone()).is_some() {
            // something bad happened, this is generally caused by a command queue being full, so try it next frame
            stop_later.push((instance, tween));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PreUpdate, cleanup_stopped_spacial_instances.in_set(AudioSystemSet::InstanceCleanup))
        .add_systems(Update, (monitor_attached_audio_sources, cleanup_despawning_audio_sources).chain())
        .add_systems(PostUpdate, (stop_audio_sources, run_spacial_audio).chain())
        .init_resource::<AttachedAudioSources>()
        .init_resource::<BufferedStopAudio>();
}
