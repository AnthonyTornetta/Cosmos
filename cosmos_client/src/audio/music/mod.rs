//! Handles the background music of cosmos

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::prelude::*;
use cosmos_core::registry::Registry;
use dynamic_music::MusicAtmosphere;

use crate::{
    audio::volume::Volume,
    settings::{Setting, SettingsRegistry, SettingsSet},
};

use super::volume::MasterVolume;

pub mod dynamic_music;

#[derive(Resource)]
struct PlayingBackgroundSong(Handle<AudioInstance>);

#[derive(Reflect, Resource, InspectorOptions, Default)]
#[reflect(Resource, InspectorOptions)]
/// Messageually this will be present for every sound type in the game, but for now this only is for music
pub struct MusicVolume(Volume);

impl MusicVolume {
    /// Returns the music volume
    pub fn get(&self) -> Volume {
        self.0
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

    instance.set_decibels(volume.get() * master_volume.get(), AudioTween::default());
}

#[derive(Message)]
/// Signals that the music system should play a background song
pub struct PlayMusicMessage {
    /// The atmosphere of the song that should be played
    pub atmosphere: MusicAtmosphere,
}

fn load_volume(settings: Res<Registry<Setting>>, mut music_volume: ResMut<MusicVolume>) {
    music_volume.0 = Volume::new(settings.i32_or("cosmos:music_volume", 100) as f32 / 100.0);
    info!("Music changed!");
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
