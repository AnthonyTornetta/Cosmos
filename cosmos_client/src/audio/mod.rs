//! Handles spacial audio a bit more nicely than bevy_kira_audio does.
//!
//! Heavily based on https://github.com/NiklasEi/bevy_kira_audio/blob/main/src/spacial.rs
//!
//! Note that this logic does rely on the `AudioReceiver` defined in kira's implementation.

use std::time::Duration;

use bevy::{
    app::{App, PostUpdate, PreUpdate, Update},
    asset::{AssetId, Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        lifecycle::RemovedComponents,
        query::{Changed, With},
        system::{Commands, Query, ResMut},
    },
    math::Vec3,
    platform::collections::HashMap,
    prelude::{Deref, DerefMut, IntoScheduleConfigs, Res, Resource, SystemSet, Transform},
    reflect::Reflect,
    transform::components::GlobalTransform,
};
use bevy_kira_audio::{AudioSystemSet, prelude::*};
use volume::MasterVolume;

use crate::audio::volume::Volume;

pub mod music;
pub mod volume;

#[derive(Reflect)]
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
    pub peak_volume: Volume,
    /// Tween used when the sound is removed from the audio emitter - Default will immediately cut it off
    #[reflect(ignore)]
    pub stop_tween: AudioTween,
    /// A weak-cloned handle that is being played. This is to prevent too many of the same audio source blowing people's ears out
    pub handle: Handle<AudioSource>,
}

impl Default for AudioEmission {
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            peak_volume: Default::default(),
            instance: Default::default(),
            handle: Default::default(),
            stop_tween: Default::default(),
        }
    }
}

#[derive(Default, Component, Reflect)]
#[require(Transform)]
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

    /// Adds an emission to this emitter
    pub fn add_emission(&mut self, emission: AudioEmission) {
        self.emissions.push(emission);
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
            }

            Some(removed)
        } else {
            None
        }
    }
}

fn run_spacial_audio(
    receiver: Query<Option<&GlobalTransform>, With<SpatialAudioReceiver>>,
    emitters: Query<(Option<&GlobalTransform>, &CosmosAudioEmitter)>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    master_volume: Res<MasterVolume>,
) {
    let Ok(receiver_transform) = receiver.single() else {
        return;
    };

    let mut num_audios_of_same_source: HashMap<(AssetId<AudioSource>, u32), (Handle<AudioInstance>, f32)> = HashMap::default();

    for (emitter_transform, emitter) in emitters.iter() {
        let (sound_path, panning) = if let Some(emitter_transform) = emitter_transform
            && let Some(receiver_transform) = receiver_transform
        {
            let sound_path = emitter_transform.translation() - receiver_transform.translation();
            let mut right_ear_angle = receiver_transform.right().angle_between(sound_path);
            // This happens if you're facing a direction that is exactly in line with whatever it is for some reason.
            if right_ear_angle.is_nan() {
                right_ear_angle = 0.0;
            }

            let panning = (right_ear_angle.cos() + 1.0) / 2.0;

            (sound_path, panning)
        } else {
            (Vec3::INFINITY, f32::NAN)
        };

        for emission in emitter.emissions.iter() {
            let Some(instance) = audio_instances.get_mut(&emission.instance) else {
                continue;
            };

            let volume = if sound_path.max_element() != f32::INFINITY {
                (emission.peak_volume * Volume::new((1.0 - sound_path.length() / emission.max_distance).clamp(0., 1.).powi(2)))
                    * master_volume.get()
            } else {
                Volume::MIN
            };

            instance.set_decibels(volume, AudioTween::default());
            if !panning.is_nan() {
                instance.set_panning(panning, AudioTween::default());
            }

            if let Some(emitter_transform) = emitter_transform
                && let PlaybackState::Playing { position } = instance.state() {
                    let pos_hashable = (position * 100.0).round() as u32;

                    let this_dist = emitter_transform.translation().length_squared();

                    if let Some((other_instance, dist)) = num_audios_of_same_source.get(&(emission.handle.id(), pos_hashable)) {
                        if this_dist >= *dist {
                            instance.stop(AudioTween::linear(Duration::from_secs(0)));
                            continue;
                        }

                        if let Some(other_instance) = audio_instances.get_mut(other_instance) {
                            other_instance.stop(AudioTween::linear(Duration::from_secs(0)));
                        }
                    }

                    num_audios_of_same_source.insert((emission.handle.id(), pos_hashable), (emission.instance.clone(), this_dist));
                }
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
        let cur_items = attached_audio_sources.0.remove(&entity).unwrap_or_default();

        let new_items = audio_emitter
            .emissions
            .iter()
            .map(|x| (x.instance.clone(), x.stop_tween.clone()))
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
            if let Some(mut ai) = audio_instances.remove(&audio_instance)
                && ai.state() != PlaybackState::Stopped
            {
                ai.stop(tween);
            }
        }
    }
}

fn cleanup_stopped_spacial_instances(
    mut emitters: Query<(Entity, &mut CosmosAudioEmitter, Option<&DespawnOnNoEmissions>)>,
    instances: ResMut<Assets<AudioInstance>>,
    mut commands: Commands,
) {
    for (entity, mut emitter, despawn_when_empty) in emitters.iter_mut() {
        let handles = &mut emitter.emissions;

        handles.retain(|emission| {
            if let Some(instance) = instances.get(&emission.instance) {
                !matches!(instance.state(), PlaybackState::Stopped)
            } else {
                false
            }
        });

        if handles.is_empty() && despawn_when_empty.is_some() {
            commands.entity(entity).despawn();
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
        instance.stop(tween.clone());
        // if instance.stop(tween.clone()).is_some() {
        // something bad happened, this is generally caused by a command queue being full, so try it next frame
        // stop_later.push((instance, tween));
        // }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// When creating spacial audio, use this set
pub enum AudioSet {
    /// Create any spacial audio bundles here
    CreateSounds,
    /// The volume will be adjusted in regards to their transforms.
    ProcessSounds,
}

pub(super) fn register(app: &mut App) {
    music::register(app);
    volume::register(app);

    app.configure_sets(Update, (AudioSet::CreateSounds, AudioSet::ProcessSounds).chain());

    app.add_systems(PreUpdate, cleanup_stopped_spacial_instances.in_set(AudioSystemSet::InstanceCleanup))
        .add_systems(PostUpdate, run_spacial_audio)
        .add_systems(
            PreUpdate,
            (stop_audio_sources, monitor_attached_audio_sources, cleanup_despawning_audio_sources)
                .before(AudioSystemSet::InstanceCleanup)
                .chain(),
        )
        .insert_resource(AudioSettings {
            sound_capacity: 8192,
            // command_capacity: 4096,
        })
        .register_type::<CosmosAudioEmitter>()
        .init_resource::<AttachedAudioSources>()
        .init_resource::<BufferedStopAudio>();
}
