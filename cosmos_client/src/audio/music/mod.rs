//! Handles the background music of cosmos

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::prelude::*;
use cosmos_core::registry::Registry;
use dynamic_music::MusicAtmosphere;

use crate::settings::{Setting, SettingsRegistry, SettingsSet};

use super::volume::MasterVolume;

pub mod dynamic_music;

#[derive(Resource)]
struct PlayingBackgroundSong(Handle<AudioInstance>);

#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
/// Messageually this will be present for every sound type in the game, but for now this only is for music
pub struct MusicVolume(#[inspector(min = 0.0, max = 1.0)] f64);

impl MusicVolume {
    /// Returns the volume as a decimal percent [0.0, 1.0]
    pub fn percent(&self) -> f64 {
        self.0.powf(2.0) * 0.2 // 1.0 is way too loud
    }
}

impl Default for MusicVolume {
    /// Initializes the volume to 1.0 (100%).
    fn default() -> Self {
        Self(1.0)
    }
}

fn monitor_background_song(
    bg_song: Res<PlayingBackgroundSong>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut commands: Commands,
) {
    if let Some(instance) = audio_instances.get_mut(&bg_song.0)
        && instance.state() == PlaybackState::Stopped
    {
        commands.remove_resource::<PlayingBackgroundSong>();
    }
}

fn adjust_volume(
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    volume: Res<MusicVolume>,
    master_volume: Res<MasterVolume>,
    background_song: Res<PlayingBackgroundSong>,
) {
    let Some(instance) = audio_instances.get_mut(&background_song.0) else {
        return;
    };

    instance.set_decibels((volume.percent() * master_volume.multiplier()) as f32, AudioTween::default());
}

#[derive(Message)]
/// Signals that the music system should play a background song
pub struct PlayMusicMessage {
    /// The atmosphere of the song that should be played
    pub atmosphere: MusicAtmosphere,
}

fn load_volume(settings: Res<Registry<Setting>>, mut music_volume: ResMut<MusicVolume>) {
    music_volume.0 = settings.i32_or("cosmos:music_volume", 100) as f64 / 100.0;
}

pub(super) fn register(app: &mut App) {
    dynamic_music::register(app);

    app.add_plugins(AudioPlugin)
        .add_systems(
            Update,
            (
                monitor_background_song.run_if(resource_exists::<PlayingBackgroundSong>),
                adjust_volume
                    .run_if(resource_changed::<MusicVolume>.or(resource_changed::<MasterVolume>))
                    .run_if(resource_exists::<PlayingBackgroundSong>),
            )
                .chain(),
        )
        .init_resource::<MusicVolume>()
        .register_type::<MusicVolume>()
        .add_message::<PlayMusicMessage>();

    app.add_systems(Update, (load_volume).in_set(SettingsSet::LoadSettings));
}
